use std::sync::Arc;

use crate::model::{
    frontier::multimodal::MultimodalFrontierConstraint, state::MultimodalStateMapping,
};

#[derive(Debug)]
pub struct MultimodalFrontierEngine {
    pub mode: String,
    pub constraints: Vec<MultimodalFrontierConstraint>,
    pub mode_to_state: Arc<MultimodalStateMapping>,
    pub route_id_to_state: Arc<MultimodalStateMapping>,
    pub max_trip_legs: u64,
    pub use_route_ids: bool,
}
