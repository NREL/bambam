use std::{collections::HashMap, sync::Arc};

use gtfs_structures::{Gtfs, RawGtfs};
use routee_compass_core::model::{
    access::{AccessModel, AccessModelError},
    map::MapModel,
    network::{Edge, EdgeId, Graph, Vertex, VertexId},
    state::{InputFeature, StateFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use rstar::RTree;

use crate::model::transit_old::transit_state_feature;

use super::{raw_gtfs_wrapper::RawGtfsWrapper, transit_edge::TransitEdge};

pub type TransitNetworkId = usize;
pub type TripId = usize;
pub type StopId = usize;

/// wraps a collection of GTFS archives and provides methods for
/// traversing the network.
///
/// # Implementation
///
/// we want to replicate the logic of OpenTripPlanner network building here,
/// which is to collect all network data, then attempt to connect all GTFS to
/// that network. in our case, the transit network
pub struct TransitTraversalModel<'a> {
    pub archives: Vec<RawGtfsWrapper<'a>>,
    pub vertex_lookup: HashMap<VertexId, Vec<(TransitNetworkId, StopId)>>,
    pub transit_edges: HashMap<EdgeId, TransitEdge>,
    pub trip_lookup: HashMap<String, TripId>,
}

impl<'a> TransitTraversalModel<'a> {
    /// reads the listed GTFS archives and tacks them onto the graph as new edges.
    ///
    /// # Returns
    ///
    /// a collection of Gtfs instances and a lookup table from VertexId into the
    /// correct Gtfs object
    pub fn new(
        urls: &[&str],
        graph: &mut Graph,
        map_model: Arc<MapModel>,
    ) -> Result<TransitTraversalModel<'a>, TraversalModelError> {
        todo!()
    }
}

impl TraversalModel for TransitTraversalModel<'_> {
    fn name(&self) -> String {
        "Transit Traversal Model".to_string()
    }

    fn input_features(&self) -> Vec<InputFeature> {
        vec![]
    }

    /// gonna need to model a bunch of things here
    /// - has a bike or a car
    /// - has a transit trip id
    /// - dist/time
    /// -
    fn output_features(&self) -> Vec<(String, StateFeature)> {
        let g = self.archives.first().unwrap();
        // let s = g.get_stop("x").unwrap();
        // let t = g.get_trip("x").unwrap();
        // let st = t.stop_times.first().unwrap();

        vec![
            transit_state_feature::transit_network_id(),
            transit_state_feature::trip_id_enumeration(),
        ]
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let (src, edge, dst) = trajectory;
        let transit_edge = self.transit_edges.get(&edge.edge_id).ok_or_else(|| {
            TraversalModelError::InternalError(format!(
                "attempting to traverse traversal edge id {} which does not exist",
                edge.edge_id
            ))
        })?;
        let gtfs = self
            .archives
            .get(transit_edge.transit_network_id)
            .ok_or_else(|| {
                TraversalModelError::InternalError(format!(
                    "traversal edge {} has gtfs archive number {} which is invalid",
                    edge.edge_id, transit_edge.transit_network_id
                ))
            })?;
        let trip = gtfs.get_trip(transit_edge.trip_id).map_err(|e| {
            TraversalModelError::InternalError(format!(
                "traversal edge {} has trip_id {} which caused failure: {}",
                edge.edge_id, transit_edge.transit_network_id, e
            ))
        })?;

        // to traverse a transit edge, we need to
        // 1. grab the trip associated with an edge
        // 2. get the route associated with the trip
        // 2. get the stop times of this trip on this edge
        todo!()
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }
}

impl AccessModel for TransitTraversalModel<'_> {
    fn state_features(&self) -> Vec<(String, StateFeature)> {
        vec![
            transit_state_feature::transit_network_id(),
            transit_state_feature::trip_id_enumeration(),
        ]
    }

    fn access_edge(
        &self,
        traversal: (&Vertex, &Edge, &Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), AccessModelError> {
        todo!()
    }
}
