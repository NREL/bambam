use crate::model::osm::{
    graph::{AdjacencyDirection, OsmGraph, OsmNodeId, OsmWayData},
    OsmError,
};
use itertools::Itertools;
use serde_json::value::Index;
use std::collections::HashSet;

/// tracks a way that will be impacted by node consolidation along with the
/// source or destination node attached to it.
pub struct WayConsolidation {
    node_id: OsmNodeId,
    orientation_from_consolidated_node: AdjacencyDirection,
    ways: Vec<OsmWayData>,
}

impl WayConsolidation {
    pub fn new(
        node_id: &OsmNodeId,
        dir: &AdjacencyDirection,
        ways: Vec<OsmWayData>,
    ) -> WayConsolidation {
        WayConsolidation {
            node_id: *node_id,
            orientation_from_consolidated_node: *dir,
            ways,
        }
    }

    pub fn get_src_dst(&self, new_node_id: &OsmNodeId) -> (OsmNodeId, OsmNodeId) {
        match self.orientation_from_consolidated_node {
            AdjacencyDirection::Forward => (*new_node_id, self.node_id),
            AdjacencyDirection::Reverse => (self.node_id, *new_node_id),
        }
    }

    pub fn drain_ways(&mut self) -> Vec<OsmWayData> {
        std::mem::take(&mut self.ways)
    }
}
