use routee_compass::{app::search::SearchAppResult, plugin::output::OutputPluginError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ActivityParameters {
    GlobalFrequencies { frequencies: HashMap<String, f64> },
}

impl ActivityParameters {
    pub fn get_frequency(
        &self,
        activity_type: &String,
        _location: Option<usize>,
        _result: &SearchAppResult,
    ) -> Result<f64, OutputPluginError> {
        match self {
            ActivityParameters::GlobalFrequencies { frequencies } => {
                let freq = frequencies.get(activity_type).ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "global frequencies missing activity type {}",
                        activity_type
                    ))
                })?;
                Ok(*freq)
            }
        }
    }
}
