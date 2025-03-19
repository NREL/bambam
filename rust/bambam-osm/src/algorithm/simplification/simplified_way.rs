use crate::model::osm::{
    graph::{
        osm_segment::OsmSegment, osm_way_data::OsmWayData, AdjacencyListDeprecated, OsmNodeId,
        OsmNodes, OsmWayId, OsmWays,
    },
    OsmError,
};
use geo::{line_string, Coord, Haversine, Length, LineString, Point};
use itertools::Itertools;
use kdam::{tqdm, Bar, BarExt};
use rayon::prelude::*;
use routee_compass_core::model::{
    network::EdgeId,
    unit::{Distance, DistanceUnit},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimplifiedWay {
    pub simplified_way_id: EdgeId,
    pub src_osmid: OsmNodeId,
    pub dst_osmid: OsmNodeId,
    pub geometry: LineString<f32>,
    pub length: Distance,
}

impl SimplifiedWay {
    pub fn new(
        src_osmid: OsmNodeId,
        dst_osmid: OsmNodeId,
        segments: Vec<OsmSegment>,
        simplified_way_id: EdgeId,
        nodes: &OsmNodes,
        ways: &OsmWays,
        distance_unit: Option<&DistanceUnit>,
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
                        "expected node {} to exist in OsmNodes collection",
                        node_id
                    ))
                })?;
                Ok(Coord::from((node.x, node.y)))
            })
            .collect::<Result<Vec<_>, OsmError>>()?;
        let geometry = LineString::new(linestring_coords);

        // find the segment length
        let length_haversine = Distance::new(geometry.length::<Haversine>() as f64);
        let length = match distance_unit {
            None => length_haversine,
            Some(du) => DistanceUnit::Meters.convert(&length_haversine, du),
        };

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
