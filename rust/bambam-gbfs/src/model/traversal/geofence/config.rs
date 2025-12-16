use serde::{Deserialize, Serialize};

/// this is where you will add the expected configuration fields required for
/// creating a GBFS service such as `gbfs_input_file: String` and any other
/// requirements for setting up GBFS for search.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GeofenceTraversalConfig {}
