use routee_compass_core::model::state::InputFeature;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(untagged, rename_all = "snake_case")]
pub struct FeatureDependencyConfig {
    /// type of the source feature dependency
    pub input_feature: InputFeature,
    /// name(s) of features that are dependent on the above input feature
    pub destination_features: Vec<(String)>,
}
