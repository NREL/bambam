use itertools::Itertools;

use crate::model::osm::{
    graph::{osm_segment::OsmSegment, OsmGraph, OsmNodeId, OsmWayId},
    OsmError,
};

#[derive(Clone, Debug, Eq)]
pub struct SimplifiedPath {
    pub src_node_id: OsmNodeId,
    pub dst_node_id: OsmNodeId,
    pub way_id: OsmWayId,
    pub path: Vec<OsmNodeId>,
    pub segments: Vec<OsmSegment>,
}

impl SimplifiedPath {
    pub fn new(
        path: Vec<OsmNodeId>,
        segments: Vec<OsmSegment>,
    ) -> Result<SimplifiedPath, OsmError> {
        match path.len() {
            0 => {
                return Err(OsmError::GraphSimplificationError(String::from(
                    "SimplifiedPath::new called with empty path",
                )))
            }
            1 => {
                return Err(OsmError::GraphSimplificationError(String::from(
                    "SimplifiedPath::new called with invalid path that only contains one node",
                )))
            }
            _ => {}
        }
        let src_node_id = *path.first().ok_or_else(|| {
            OsmError::InternalError(String::from("non-empty path has no source node"))
        })?;
        let dst_node_id = *path.last().ok_or_else(|| {
            OsmError::InternalError(String::from("non-empty path has no destination node"))
        })?;
        let way_id = segments
            .first()
            .ok_or_else(|| {
                OsmError::GraphSimplificationError(String::from(
                    "SimplifiedPath created with zero segments",
                ))
            })?
            .way_id;
        Ok(SimplifiedPath {
            src_node_id,
            dst_node_id,
            way_id,
            path,
            segments,
        })
    }

    /// adds this new simplified path to the graph as a segment
    pub fn add_path_to_graph(&self, graph: &mut OsmGraph) -> Result<(), OsmError> {
        /// adds a path between two nodes as a single segment
        // graph.add(self.src_node_id, self.dst_node_id, self.segments.clone())?;
        Ok(())
    }

    /// remove all but the src and dst nodes for this path.
    /// if the node is not found, do not fail. this is because the original code
    /// in osmnx removes nodes exactly once by first making the node list a set:
    ///     G.remove_nodes_from(set(all_nodes_to_remove))
    pub fn remove_interstitial_nodes(&self, graph: &mut OsmGraph) -> Result<(), OsmError> {
        for node_id in self.path.iter().dropping(1).dropping_back(1) {
            graph.disconnect_node(node_id, false)?;
        }
        Ok(())
    }
}

impl PartialEq for SimplifiedPath {
    fn eq(&self, other: &Self) -> bool {
        if self.segments == other.segments {
            true
        } else {
            for (a, b) in self.path.iter().zip(other.path.iter()) {
                if a != b {
                    return false;
                }
            }
            true
        }
    }
}

impl PartialOrd for SimplifiedPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.path.partial_cmp(&other.path) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.segments.partial_cmp(&other.segments)
    }
}
