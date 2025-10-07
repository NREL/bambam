use super::MultimodalFrontierConstraintConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalFrontierConfig {
    /// name of the mode associated with this edge list
    pub mode: String,
    /// constraints to apply when in this mode
    pub constraints: Vec<MultimodalFrontierConstraintConfig>,
    /// modes that can be used on this trip
    pub available_modes: Vec<String>,
    /// route ids that can be used on this trip
    pub available_route_ids: Vec<String>,
    /// true if this edge list uses route ids
    pub use_route_ids: bool,
    /// maximum number of legs allowed in a trip
    pub max_trip_legs: u64,
}
