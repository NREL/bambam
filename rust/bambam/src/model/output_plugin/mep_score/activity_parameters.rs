use routee_compass::{
    app::{compass::CompassComponentError, search::SearchAppResult},
    plugin::{output::OutputPluginError, PluginError},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::output_plugin::mep_score::ActivityParametersConfig;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ActivityParameters {
    GlobalFrequencies {
        frequencies: HashMap<String, f64>,
        frequency_sum: f64,
    },
}

impl TryFrom<&ActivityParametersConfig> for ActivityParameters {
    type Error = CompassComponentError;

    fn try_from(value: &ActivityParametersConfig) -> Result<Self, Self::Error> {
        match value {
            ActivityParametersConfig::GlobalFrequencies { frequencies } => {
                let frequency_sum: f64 = frequencies.values().sum();
                if frequency_sum == 0.0 {
                    let err: PluginError = OutputPluginError::BuildFailed(String::from(
                        "sum of activity frequencies cannot be zero",
                    ))
                    .into();
                    Err(err.into())
                } else {
                    Ok(ActivityParameters::GlobalFrequencies {
                        frequencies: frequencies.clone(),
                        frequency_sum,
                    })
                }
            }
        }
    }
}

impl ActivityParameters {
    pub fn get_frequency_term(
        &self,
        activity_type: &str,
        _location: Option<&geo::Point<f32>>,
    ) -> Result<f64, OutputPluginError> {
        match self {
            ActivityParameters::GlobalFrequencies {
                frequencies,
                frequency_sum,
            } => {
                let freq = frequencies.get(activity_type).ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "global frequencies missing activity type {}",
                        activity_type
                    ))
                })?;

                Ok(*freq / *frequency_sum)
            }
        }
    }
}
