use super::transit_traversal_model::{TransitNetworkId, TripId};
use routee_compass_core::model::network::{EdgeId, VertexId};

pub struct TransitTrip {
    pub transit_network_id: TransitNetworkId,
    pub trip_id: usize,
    pub route_id: usize,
    // pub
}
