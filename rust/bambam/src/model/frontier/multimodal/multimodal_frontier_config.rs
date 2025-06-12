use super::MultimodalConstraintConfig;
use routee_compass_core::model::network::EdgeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalFrontierConfig {
    /// constraints to apply when in this mode
    constraints: HashMap<String, MultimodalConstraintConfig>,
    /// lists modes with access to the scheduled network. these modes will be able
    /// to traverse edges with EdgeId greater than $scheduled_edge_boundary.
    scheduled_modes: Vec<String>,
    /// will we need this? something to mark what number scheduled EdgeIds begin at.
    /// could it be passed to the FrontierModelBuilder after computing it dynamically
    /// during CompassApp or SearchApp construction?
    scheduled_edge_boundary: EdgeId,
}
