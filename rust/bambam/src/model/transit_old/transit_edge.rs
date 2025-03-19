use routee_compass_core::model::network::{EdgeId, VertexId};

use super::transit_traversal_model::{TransitNetworkId, TripId};

pub struct TransitEdge {
    pub destination: VertexId,
    pub transit_network_id: TransitNetworkId,
    pub trip_id: usize,
    pub src_stop_time_id: usize,
    pub dst_stop_time_id: usize,
}
