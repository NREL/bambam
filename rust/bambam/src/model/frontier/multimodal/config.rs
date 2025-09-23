use crate::model::state::MultimodalMapping;

use super::MultimodalFrontierConstraintConfig;
use routee_compass_core::model::network::EdgeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalFrontierConfig {
    /// constraints to apply when in this mode
    pub constraints: HashMap<String, MultimodalFrontierConstraintConfig>,
    /// enumerates modes as integers in the state vector
    pub mode_to_state: Vec<String>,
    /// modes that can be used on this trip
    pub available_modes: Vec<String>,
}
