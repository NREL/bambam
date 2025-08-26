use serde::{Deserialize, Serialize};
use crate::model::output_plugin::opportunity::{OpportunityFormat, OpportunityModelConfig};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpportunityPluginConfig {
    pub model: OpportunityModelConfig,
    pub collect_format: OpportunityFormat
}