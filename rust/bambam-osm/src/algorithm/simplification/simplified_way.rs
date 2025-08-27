use crate::model::osm::{
    graph::{osm_segment::OsmSegment, OsmNodeId, OsmNodes, OsmWays},
    OsmError,
};
use geo::{line_measures::LengthMeasurable, Convert, Coord, Haversine, Length, LineString};
use itertools::Itertools;
use rayon::prelude::*;
use routee_compass_core::model::network::EdgeId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimplifiedWay {
    pub simplified_way_id: EdgeId,
    pub src_osmid: OsmNodeId,
    pub dst_osmid: OsmNodeId,
    pub geometry: LineString<f32>,
    pub length: uom::si::f64::Length,
}

impl SimplifiedWay {
    pub fn new(
        src_osmid: OsmNodeId,
        dst_osmid: OsmNodeId,
        segments: Vec<OsmSegment>,
        simplified_way_id: EdgeId,
        nodes: &OsmNodes,
        ways: &OsmWays,
    ) -> Result<SimplifiedWay, OsmError> {
        // grab all original ways associated with this aggregated segment
        let segment_ways = segments
            .iter()
            .map(|s| {
                ways.get(&s.way_id).ok_or_else(|| {
                    OsmError::GraphSimplificationError(format!(
                        "during simplification, unable to find way {} in ways collection",
                        s.way_id
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // create a LineString
        let mut linestring_node_ids: Vec<&OsmNodeId> = segment_ways
            .iter()
            .flat_map(|way| way.nodes.iter())
            .collect_vec();
        linestring_node_ids.dedup();
        let linestring_coords: Vec<Coord<f32>> = linestring_node_ids
            .into_iter()
            .map(|node_id| {
                let node = nodes.get(node_id).ok_or_else(|| {
                    OsmError::GraphSimplificationError(format!(
                        "expected node {node_id} to exist in OsmNodes collection"
                    ))
                })?;
                Ok(Coord::from((node.x, node.y)))
            })
            .collect::<Result<Vec<_>, OsmError>>()?;
        let geometry = LineString::new(linestring_coords);

        let geometry_f64: LineString<f64> = geometry.convert(); // use f64 precision in haversine
        let length_f64 = Haversine.length(&geometry_f64);
        let length = uom::si::f64::Length::new::<uom::si::length::meter>(length_f64);

        let way = SimplifiedWay {
            simplified_way_id,
            src_osmid,
            dst_osmid,
            geometry,
            length,
        };
        Ok(way)
    }
}
