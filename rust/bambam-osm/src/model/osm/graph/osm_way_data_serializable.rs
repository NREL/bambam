use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use geo::{Coord, Haversine, Length, LineString};
use itertools::Itertools;
use routee_compass_core::model::{
    network::{Vertex, VertexId},
    unit::{Distance, Grade, Speed, SpeedUnit},
};
use serde::{Deserialize, Serialize, Serializer};
use wkt::ToWkt;

use crate::model::{
    feature::highway::{self, Highway},
    osm::OsmError,
};

use super::{OsmGraph, OsmNodeData, OsmNodeId, OsmNodes, OsmSegment, OsmWayData, OsmWayId};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OsmWayDataSerializable {
    pub osmid: OsmWayId,
    pub src_vertex_id: VertexId,
    pub dst_vertex_id: VertexId,
    pub nodes: Option<String>,
    pub access: Option<String>,
    pub area: Option<String>,
    pub bridge: Option<String>,
    pub est_width: Option<String>,
    pub highway: Highway,
    pub sidewalk: Option<String>,
    pub footway: Option<String>,
    pub junction: Option<String>,
    pub landuse: Option<String>,
    pub lanes: Option<String>,
    pub maxspeed: Option<String>,
    pub maxspeed_raw: Option<String>,
    pub speed_kph: Option<String>,
    pub speed_kph_raw: Option<String>,
    pub name: Option<String>,
    pub oneway: Option<String>,
    pub _ref: Option<String>,
    pub service: Option<String>,
    pub tunnel: Option<String>,
    pub width: Option<String>,
    /// when ways are simplified, the list of composite way ids are stored here.
    /// the Way.osmid will remain present in any aggregate way_ids collection.
    pub way_ids: Option<String>,
    #[serde(serialize_with = "serialize_linestring")]
    pub linestring: LineString<f32>,
    pub length_meters: f32,
}

impl OsmWayDataSerializable {
    const DEFAULT_WALK_SPEED_KPH: f64 = 5.0;
    /// a delimter for aggregated fields which does not collide with CSV delimiters
    pub const VALUE_DELIMITER: &'static str = ";";
}

impl OsmWayDataSerializable {
    /// creates a new output row from triplets that each represent a multiedge adjacency
    /// in the fully-processed multiedge graph.
    ///
    /// here we finally deal with aggregating fields where possible.
    pub fn new(
        triplets: Vec<(&OsmNodeData, &OsmWayData, &OsmNodeData)>,
        graph: &OsmGraph,
        vertex_lookup: &HashMap<OsmNodeId, (usize, Vertex)>,
    ) -> Result<Self, OsmError> {
        // prevent building invalid linestring objects with 0 or 1 coordinates
        let nodes_connected = triplets
            .iter()
            .flat_map(|(_, e, _)| e.nodes.clone())
            .collect::<HashSet<_>>();
        if nodes_connected.len() < 2 {
            return Err(OsmError::InternalError(String::from(
                "attempting to build output row with fewer than 2 unique nodes/coordinates present",
            )));
        }

        // in OSMNx, the first edge in a multi-edge is the one that is taken.
        // but perhaps we should consider combining edges here with OsmWayData::try_from(ways.as_slice())?
        // note from osmnx.simplification:
        //
        // # get edge between these nodes: if multiple edges exist between
        // # them (see above), we retain only one in the simplified graph
        // # We can't assume that there exists an edge from u to v
        // # with key=0, so we get a list of all edges from u to v
        // # and just take the first one.
        let (src_node, way, dst_node) = triplets.into_iter().next().ok_or_else(|| {
            OsmError::InternalError(String::from(
                "attempting to build output row from empty trajectory",
            ))
        })?;
        let src_node_id = src_node.osmid;
        let dst_node_id = dst_node.osmid;

        let (src_vertex_id, _) = &vertex_lookup.get(&src_node_id).ok_or_else(|| {
            OsmError::InternalError(format!(
                "during output processing, way ({})-[{}]->({}) has no matching source vertex id",
                src_node_id, way.osmid, dst_node_id
            ))
        })?;
        let (dst_vertex_id, _) = &vertex_lookup.get(&dst_node_id).ok_or_else(|| {
            OsmError::InternalError(format!(
                "during output processing, way ({})-[{}]->({}) has no matching destination vertex id",
                src_node_id, way.osmid, dst_node_id
            ))
        })?;

        let linestring = create_linestring_for_od_path(&src_node_id, &dst_node_id, way, graph)?;
        if linestring.coords().collect_vec().len() < 2 {
            return Err(OsmError::InternalError(format!(
                "during output processing, way ({})-[{}]->({}) produces a linestring with less than 2 nodes: '{}'",
                src_node_id, way.osmid, dst_node_id, linestring.to_wkt()
            )));
        }
        let length_meters = Haversine.length(&linestring);
        let highway = top_highway(&way.highway, OsmWayData::VALUE_DELIMITER)?;

        let row = Self {
            osmid: way.osmid,
            src_vertex_id: VertexId(*src_vertex_id),
            dst_vertex_id: VertexId(*dst_vertex_id),
            highway,
            linestring,
            length_meters,
            // NUMERICAL
            area: max(way.area.as_ref()),
            est_width: min(way.est_width.as_ref()),
            lanes: min(way.lanes.as_ref()),
            maxspeed: min(way.maxspeed.as_ref()),
            speed_kph: min(way.speed_kph.as_ref()),
            width: min(way.width.as_ref()),
            // CATEGORICAL / RAW / REFERENCE
            _ref: unique(way._ref.as_ref()),
            access: unique(way.access.as_ref()),
            bridge: unique(way.bridge.as_ref()),
            footway: unique(way.footway.as_ref()),
            junction: unique(way.junction.as_ref()),
            landuse: unique(way.landuse.as_ref()),
            maxspeed_raw: replace_delimiter(way.maxspeed.as_ref()),
            name: unique(way.name.as_ref()),
            nodes: join_node_ids(&way.nodes),
            oneway: unique(way.oneway.as_ref()),
            sidewalk: unique(way.sidewalk.as_ref()),
            service: unique(way.service.as_ref()),
            speed_kph_raw: replace_delimiter(way.speed_kph.as_ref()),
            tunnel: unique(way.tunnel.as_ref()),
            way_ids: join_way_ids(way.way_ids.as_ref()),
        };
        Ok(row)
    }

    pub fn get_string_at_field(&self, fieldname: &str) -> Result<Option<String>, String> {
        match fieldname {
            "access" => Ok(self.access.clone()),
            "area" => Ok(self.area.clone()),
            "bridge" => Ok(self.bridge.clone()),
            "est_width" => Ok(self.est_width.clone()),
            "highway" => Ok(Some(self.highway.to_string())),
            "sidewalk" => Ok(self.sidewalk.clone()),
            "footway" => Ok(self.footway.clone()),
            "junction" => Ok(self.junction.clone()),
            "landuse" => Ok(self.landuse.clone()),
            "lanes" => Ok(self.lanes.clone()),
            "maxspeed" => Ok(self.maxspeed.clone()),
            "speed_kph" => Ok(self.speed_kph.clone()),
            "name" => Ok(self.name.clone()),
            "oneway" => Ok(self.oneway.clone()),
            "ref" => Ok(self._ref.clone()),
            "service" => Ok(self.service.clone()),
            "tunnel" => Ok(self.tunnel.clone()),
            "width" => Ok(self.width.clone()),
            _ => Err(format!("unknown edge field {fieldname}")),
        }
    }

    /// follows the rules described in
    /// https://wiki.openstreetmap.org/wiki/Key:maxspeed#Values
    pub fn get_speed(
        &self,
        key: &str,
        ignore_invalid_entries: bool,
    ) -> Result<Option<(Speed, SpeedUnit)>, String> {
        match self.get_string_at_field(key) {
            Ok(None) => Ok(None),
            Ok(Some(s)) => deserialize_speed(&s, ignore_invalid_entries),
            Err(e) => Err(e),
        }
    }
}

/// shorten the value, assumed a delimited string of categoricals, so that
/// it contains only the unique set of categories.
fn unique(value: Option<&String>) -> Option<String> {
    let split = value.map(|v| v.split(OsmWayData::VALUE_DELIMITER).collect_vec());

    match split {
        None => None,
        Some(values) if values.is_empty() => None,
        Some(mut nonempty_values) => {
            nonempty_values.dedup();
            let result = nonempty_values.join(OsmWayDataSerializable::VALUE_DELIMITER);
            Some(result)
        }
    }
}

/// take the minimum numerical value present in the input rows, with the following algorithm:
///   - if numbers can be all integers, find the min
///   - else if numbers can be all floating point, find the min
///   - else return whatever value already is with delimiter replaced for CSV
fn min(value: Option<&String>) -> Option<String> {
    let split = value
        .map(|v| v.split(OsmWayData::VALUE_DELIMITER).collect_vec())
        .unwrap_or_default();
    let parsed_min_i64 = as_parsed(&split, |vs: &[i64]| vs.iter().cloned().min());
    parsed_min_i64.or_else(|| {
        as_parsed(&split, |vs: &[f64]| {
            vs.iter()
                .cloned()
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
    })
}

/// take the maximum numerical value present in the input rows, with the following algorithm:
///   - if numbers can be all integers, find the max
///   - else if numbers can be all floating point, find the max
///   - else return whatever value already is with delimiter replaced for CSV
fn max(value: Option<&String>) -> Option<String> {
    let split = value
        .map(|v| v.split(OsmWayData::VALUE_DELIMITER).collect_vec())
        .unwrap_or_default();
    let parsed_max_i64 = as_parsed(&split, |vs: &[i64]| vs.iter().cloned().max());
    parsed_max_i64.or_else(|| {
        as_parsed(&split, |vs: &[f64]| {
            vs.iter()
                .cloned()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
    })
}

/// parses the values as a list of T and aggregates over those T to produce a single value
/// which is then serialized again to string.
fn as_parsed<T, F>(values: &[&str], agg: F) -> Option<String>
where
    T: FromStr + std::fmt::Display,
    F: FnOnce(&[T]) -> Option<T>,
{
    let as_t = values
        .iter()
        .map(|s| s.parse::<T>())
        .collect::<Result<Vec<_>, _>>();
    match as_t {
        Ok(ts) => {
            let agg_result = agg(&ts);
            agg_result.map(|agg_t| format!("{agg_t}"))
        }
        Err(_) => None,
    }
}

fn replace_delimiter(value: Option<&String>) -> Option<String> {
    value.as_ref().map(|v| {
        v.replace(
            OsmWayData::VALUE_DELIMITER,
            OsmWayDataSerializable::VALUE_DELIMITER,
        )
    })
}

fn join_node_ids(value: &[OsmNodeId]) -> Option<String> {
    match value[..] {
        [] => None,
        _ => {
            let joined = value
                .iter()
                .map(|id| format!("{id}"))
                .join(OsmWayDataSerializable::VALUE_DELIMITER);
            Some(joined)
        }
    }
}

fn join_way_ids(value: &[OsmWayId]) -> Option<String> {
    match value[..] {
        [] => None,
        _ => {
            let joined = value
                .iter()
                .map(|id| format!("{id}"))
                .join(OsmWayDataSerializable::VALUE_DELIMITER);
            Some(joined)
        }
    }
}

pub fn create_linestring_for_od_path(
    src: &OsmNodeId,
    dst: &OsmNodeId,
    way: &OsmWayData,
    graph: &OsmGraph,
) -> Result<LineString<f32>, OsmError> {
    let coords = extract_between_nodes(src, dst, &way.nodes)
        .ok_or_else(|| {
            let nodes = way.nodes.iter().map(|n| format!("({n})")).join("->");
            OsmError::InternalError(format!(
                "trajectory ({})-[{}]->({}) not found in (aggregate) way nodes: {}",
                src, way.osmid, dst, nodes
            ))
        })?
        .iter()
        .map(|n| {
            let node = graph.get_node_data(n)?;
            Ok(Coord::from((node.x, node.y)))
        })
        .collect::<Result<Vec<Coord<f32>>, _>>()?;
    Ok(LineString(coords))
}

/// if the highway value is non-empty, split it by the expected delimiter and take the top-ranked Highway
/// tag by it's Highway::hierarchy().
fn top_highway(
    highway_value: &Option<String>,
    delimiter: &'static str,
) -> Result<Highway, OsmError> {
    match highway_value {
        None => Err(OsmError::InternalError(String::from(
            "output Way has no Highway key",
        ))),
        Some(h_str) => {
            let tags = h_str
                .split(delimiter)
                .map(|h| {
                    Highway::from_str(h).map_err(|e| {
                        OsmError::InvalidOsmData(format!("found invalid highway tag {e}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let highway = tags
                .into_iter()
                .max_by_key(|t| t.hierarchy())
                .ok_or_else(|| {
                    OsmError::InternalError(String::from(
                        "non-empty row Highway tag has empty set of tags",
                    ))
                })?;
            Ok(highway)
        }
    }
}

/// deals with the various ways that speed keys can appear. handles
/// valid cases such as:
///   - 45        (45 kph)
///   - 45 mph    (72.4203 kph)
///   - walk      (5 kph)
///
/// and invalid cases that are documented, such as:
///   - 45; 80    (takes the smaller of the two, so, 45 kph)
///
/// see https://wiki.openstreetmap.org/wiki/Key:maxspeed
fn deserialize_speed(
    s: &str,
    ignore_invalid_entries: bool,
) -> Result<Option<(Speed, SpeedUnit)>, String> {
    let separated_entries = s.split([',', ';']).collect_vec();
    match separated_entries[..] {
        [] => Err(format!(
            "internal error: attempting to unpack empty maxspeed value '{s}'"
        )),
        [entry] => {
            match entry.split(" ").collect_vec()[..] {
                // see https://wiki.openstreetmap.org/wiki/Key:maxspeed#Possible_tagging_mistakes
                // for list of some values we should ignore that are known.
                ["unposted"] => Ok(None),
                ["unknown"] => Ok(None),
                ["default"] => Ok(None),
                ["variable"] => Ok(None),
                ["national"] => Ok(None),
                ["25mph"] => Ok(Some((Speed::from(25.0), SpeedUnit::MPH))),

                // todo! handle all default speed limits
                // see https://wiki.openstreetmap.org/wiki/Default_speed_limits
                ["walk"] => {
                    // Austria + Germany's posted "walking speed". i found a reference that
                    // suggests this is 4-7kph:
                    // https://en.wikivoyage.org/wiki/Driving_in_Germany#Speed_limits
                    Ok(Some((
                        Speed::from(OsmWayDataSerializable::DEFAULT_WALK_SPEED_KPH),
                        SpeedUnit::KPH,
                    )))
                }
                [speed_str] => {
                    let speed_result = speed_str
                        .parse::<f64>()
                        .map_err(|e| format!("speed value {speed_str} not a valid number: {e}"));

                    let speed = match speed_result {
                        Ok(speed) => speed,
                        Err(e) if !ignore_invalid_entries => {
                            return Err(e);
                        }
                        Err(_) => return Ok(None),
                    };
                    if speed == 0.0 {
                        Ok(None)
                    } else {
                        Ok(Some((Speed::from(speed), SpeedUnit::KPH)))
                    }
                }
                [speed_str, unit_str] => {
                    let speed_result = speed_str
                        .parse::<f64>()
                        .map_err(|e| format!("speed value {speed_str} not a valid number: {e}"));

                    let speed = match speed_result {
                        Ok(speed) => speed,
                        Err(e) if !ignore_invalid_entries => {
                            return Err(e);
                        }
                        Err(_) => return Ok(None),
                    };
                    if speed == 0.0 {
                        return Ok(None);
                    }
                    let speed_unit = match unit_str {
                        "kph" => SpeedUnit::KPH,
                        "mph" => SpeedUnit::MPH,
                        _ if !ignore_invalid_entries => {
                            return Err(format!(
                                "unknown speed unit {unit_str} with value {speed}"
                            ));
                        }
                        _ => {
                            // some garbage or uncommon unit type like feet per minute, we can skip this entry.
                            return Ok(None);
                        }
                    };
                    let result = (Speed::from(speed), speed_unit);
                    Ok(Some(result))
                }
                _ => Err(format!("unexpected maxspeed entry '{s}'")),
            }
        }
        _ => {
            let maxspeeds = separated_entries
                .to_vec()
                .iter()
                .map(|e| deserialize_speed(e, ignore_invalid_entries))
                .collect::<Result<Vec<_>, _>>()?;
            let min = maxspeeds
                .into_iter()
                .min_by_key(|m| match m {
                    Some((s, _)) => *s,
                    None => Speed::from(999999.9),
                })
                .flatten();
            Ok(min)
        }
    }
}

fn serialize_linestring<S>(row: &LineString<f32>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let wkt = row.to_wkt().to_string();
    s.serialize_str(&wkt)
}

fn extract_between_nodes<'a>(
    src: &'a OsmNodeId,
    dst: &'a OsmNodeId,
    nodes: &'a [OsmNodeId],
) -> Option<&'a [OsmNodeId]> {
    let start = nodes.iter().position(|x| x == src)?; // Using ? for early return
    let end = nodes[start..].iter().position(|x| x == dst)?; // Search after 'a'

    if start <= start + end {
        Some(&nodes[start..=start + end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use routee_compass_core::model::unit::{AsF64, SpeedUnit};

    use super::extract_between_nodes;
    use crate::model::osm::graph::OsmNodeId;

    #[test]
    fn test_extract() {
        let nodes = vec![
            OsmNodeId(1),
            OsmNodeId(2),
            OsmNodeId(3),
            OsmNodeId(4),
            OsmNodeId(5),
            OsmNodeId(6),
        ];
        let result = extract_between_nodes(&OsmNodeId(2), &OsmNodeId(4), &nodes);
        println!("{result:?}");
        let expected = [&OsmNodeId(2), &OsmNodeId(3), &OsmNodeId(4)];
        match result {
            Some([a, b, c]) => {
                assert_eq!(a, &nodes[1]);
                assert_eq!(b, &nodes[2]);
                assert_eq!(c, &nodes[3]);
            }
            _ => panic!("not as expected"),
        }
    }

    #[test]
    fn deserialize_speed_1() {
        //   - 45        (45 kph)
        match super::deserialize_speed("45", false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 45.0);
                assert_eq!(speed_unit, SpeedUnit::KPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }
    #[test]
    fn deserialize_speed_2() {
        //   - 45 mph    (72.4203 kph)
        match super::deserialize_speed("45 mph", false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 45.0);
                assert_eq!(speed_unit, SpeedUnit::MPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }
    #[test]
    fn deserialize_speed_3() {
        //   - walk      (5 kph)
        match super::deserialize_speed("5 kph", false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 5.0);
                assert_eq!(speed_unit, SpeedUnit::KPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }
}
