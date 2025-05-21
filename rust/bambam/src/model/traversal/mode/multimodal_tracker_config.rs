use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalTrackerConfig {
    pub name: String,
}
