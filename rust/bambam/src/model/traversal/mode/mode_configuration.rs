use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModeConfiguration {
    /// name of the travel mode
    pub name: String,
    /// speed feature name on the state model to use for speed values
    pub speed_feature: String,
}
