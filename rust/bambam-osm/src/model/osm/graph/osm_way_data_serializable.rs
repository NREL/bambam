use std::{collections::HashMap, str::FromStr};

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
    pub junction: Option<String>,
    pub landuse: Option<String>,
    pub lanes: Option<String>,
    pub maxspeed: Option<String>,
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
    pub fn new(
        traj: Vec<(&OsmNodeData, &OsmWayData, &OsmNodeData)>,
        graph: &OsmGraph,
        vertex_lookup: &HashMap<OsmNodeId, (usize, Vertex)>,
    ) -> Result<Self, OsmError> {
        // in OSMNx, the first edge in a multi-edge is the one that is taken.
        // but perhaps we should consider combining edges here with OsmWayData::try_from(ways.as_slice())?
        // note from osmnx.simplification:
        //
        // # get edge between these nodes: if multiple edges exist between
        // # them (see above), we retain only one in the simplified graph
        // # We can't assume that there exists an edge from u to v
        // # with key=0, so we get a list of all edges from u to v
        // # and just take the first one.
        let (src_node, way, dst_node) = traj.into_iter().next().ok_or_else(|| {
            OsmError::InternalError(String::from(
                "attempting to build output row for adjacency triplet with no ways",
            ))
        })?;
        let src_node_id = src_node.osmid;
        let dst_node_id = dst_node.osmid;

        let (src_vertex_id, _) = &vertex_lookup.get(&src_node_id).ok_or_else(|| {
            OsmError::InternalError(format!(
                "during output processing, way ({})-[{}]->({}) has no matching vertex id",
                src_node_id, way.osmid, dst_node_id
            ))
        })?;
        let (dst_vertex_id, _) = &vertex_lookup.get(&dst_node_id).ok_or_else(|| {
            OsmError::InternalError(format!(
                "during output processing, way ({})-[{}]->({}) has no matching vertex id",
                src_node_id, way.osmid, dst_node_id
            ))
        })?;

        let linestring = create_linestring_for_od_path(&src_node_id, &dst_node_id, way, graph)?;
        let length_meters = Haversine.length(&linestring);
        let highway = top_highway(&way.highway, OsmWayData::VALUE_DELIMITER)?;
        let row = Self {
            osmid: way.osmid,
            src_vertex_id: VertexId(*src_vertex_id),
            dst_vertex_id: VertexId(*dst_vertex_id),
            nodes: join_node_ids(&way.nodes, Self::VALUE_DELIMITER),
            access: replace_delimiter(&way.access, Self::VALUE_DELIMITER),
            area: replace_delimiter(&way.area, Self::VALUE_DELIMITER),
            bridge: replace_delimiter(&way.bridge, Self::VALUE_DELIMITER),
            est_width: replace_delimiter(&way.est_width, Self::VALUE_DELIMITER),
            highway,
            junction: replace_delimiter(&way.junction, Self::VALUE_DELIMITER),
            landuse: replace_delimiter(&way.landuse, Self::VALUE_DELIMITER),
            lanes: replace_delimiter(&way.lanes, Self::VALUE_DELIMITER),
            maxspeed: replace_delimiter(&way.maxspeed, Self::VALUE_DELIMITER),
            name: replace_delimiter(&way.name, Self::VALUE_DELIMITER),
            oneway: replace_delimiter(&way.oneway, Self::VALUE_DELIMITER),
            _ref: replace_delimiter(&way._ref, Self::VALUE_DELIMITER),
            service: replace_delimiter(&way.service, Self::VALUE_DELIMITER),
            tunnel: replace_delimiter(&way.tunnel, Self::VALUE_DELIMITER),
            width: replace_delimiter(&way.width, Self::VALUE_DELIMITER),
            way_ids: join_way_ids(&way.way_ids, Self::VALUE_DELIMITER),
            linestring,
            length_meters,
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
            "junction" => Ok(self.junction.clone()),
            "landuse" => Ok(self.landuse.clone()),
            "lanes" => Ok(self.lanes.clone()),
            "maxspeed" => Ok(self.maxspeed.clone()),
            "name" => Ok(self.name.clone()),
            "oneway" => Ok(self.oneway.clone()),
            "ref" => Ok(self._ref.clone()),
            "service" => Ok(self.service.clone()),
            "tunnel" => Ok(self.tunnel.clone()),
            "width" => Ok(self.width.clone()),
            _ => Err(format!("unknown edge field {}", fieldname)),
        }
    }

    /// follows the rules described in
    /// https://wiki.openstreetmap.org/wiki/Key:maxspeed#Values
    pub fn get_maxspeed(
        &self,
        ignore_invalid_entries: bool,
    ) -> Result<Option<(Speed, SpeedUnit)>, String> {
        match self.get_string_at_field("maxspeed") {
            Ok(None) => Ok(None),
            Ok(Some(s)) => deserialize_maxspeed(&s, ignore_invalid_entries),
            Err(e) => Err(e),
        }
    }
}

fn replace_delimiter(value: &Option<String>, delimiter: &'static str) -> Option<String> {
    value
        .as_ref()
        .map(|v| v.replace(OsmWayData::VALUE_DELIMITER, delimiter))
}

fn join_node_ids(value: &Vec<OsmNodeId>, delimiter: &'static str) -> Option<String> {
    match value[..] {
        [] => None,
        _ => {
            let joined = value.iter().map(|id| format!("{}", id)).join(delimiter);
            Some(joined)
        }
    }
}

fn join_way_ids(value: &Vec<OsmWayId>, delimiter: &'static str) -> Option<String> {
    match value[..] {
        [] => None,
        _ => {
            let joined = value.iter().map(|id| format!("{}", id)).join(delimiter);
            Some(joined)
        }
    }
}

fn create_linestring_for_od_path(
    src: &OsmNodeId,
    dst: &OsmNodeId,
    way: &OsmWayData,
    graph: &OsmGraph,
) -> Result<LineString<f32>, OsmError> {
    let coords = extract_between_nodes(src, dst, &way.nodes)
        .ok_or_else(|| {
            let nodes = way.nodes.iter().map(|n| format!("({})", n)).join("->");
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
                        OsmError::InvalidOsmData(format!("found invalid highway tag {}", e))
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

/// deals with the various ways that the maxspeed key can appear. handles
/// valid cases such as:
/// - 45        (45 kph)
/// - 45 mph    (72.4203 kph)
/// - walk      (5 kph)
/// and invalid cases that are documented, such as:
/// - 45; 80    (takes the smaller of the two, so, 45 kph)
/// see https://wiki.openstreetmap.org/wiki/Key:maxspeed
fn deserialize_maxspeed(
    s: &str,
    ignore_invalid_entries: bool,
) -> Result<Option<(Speed, SpeedUnit)>, String> {
    let separated_entries = s.split([',', ';']).collect_vec();
    match separated_entries[..] {
        [] => Err(format!(
            "internal error: attempting to unpack empty maxspeed value '{}'",
            s
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
                    let speed_result = speed_str.parse::<f64>().map_err(|e| {
                        format!("speed value {} not a valid number: {}", speed_str, e)
                    });

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
                    let speed_result = speed_str.parse::<f64>().map_err(|e| {
                        format!("speed value {} not a valid number: {}", speed_str, e)
                    });

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
                                "unknown speed unit {} with value {}",
                                unit_str, speed
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
                _ => Err(format!("unexpected maxspeed entry '{}'", s)),
            }
        }
        _ => {
            let maxspeeds = separated_entries
                .to_vec()
                .iter()
                .map(|e| deserialize_maxspeed(e, ignore_invalid_entries))
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
    nodes: &'a Vec<OsmNodeId>,
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
        println!("{:?}", result);
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
}
