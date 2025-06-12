use super::{FeatureDependency, FeatureMappingsByMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct MultimodalTraversalConfig {
    /// for each mode, the mapping of feature names to their role
    /// in mode-specific state updates and search cost aggregation.
    pub modes: FeatureMappingsByMode,
}
