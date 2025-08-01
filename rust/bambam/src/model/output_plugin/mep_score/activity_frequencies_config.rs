use routee_compass::{app::search::SearchAppResult, plugin::output::OutputPluginError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ActivityFrequenciesConfig {
    GlobalFrequencies { frequencies: HashMap<String, f64> },
}
