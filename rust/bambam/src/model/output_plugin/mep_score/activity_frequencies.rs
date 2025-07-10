use routee_compass::{
    app::{compass::CompassComponentError, search::SearchAppResult},
    plugin::{output::OutputPluginError, PluginError},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::output_plugin::mep_score::ActivityFrequenciesConfig;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ActivityFrequencies {
    GlobalFrequencies {
        frequencies: HashMap<String, f64>,
        frequency_sum: f64,
    },
}

impl TryFrom<&ActivityFrequenciesConfig> for ActivityFrequencies {
    type Error = CompassComponentError;

    fn try_from(value: &ActivityFrequenciesConfig) -> Result<Self, Self::Error> {
        match value {
            ActivityFrequenciesConfig::GlobalFrequencies { frequencies } => {
                let frequency_sum: f64 = frequencies.values().sum();
                if frequency_sum == 0.0 {
                    let err: PluginError = OutputPluginError::BuildFailed(String::from(
                        "sum of activity frequencies cannot be zero",
                    ))
                    .into();
                    Err(err.into())
                } else {
                    Ok(ActivityFrequencies::GlobalFrequencies {
                        frequencies: frequencies.clone(),
                        frequency_sum,
                    })
                }
            }
        }
    }
}

impl ActivityFrequencies {
    pub fn get_frequency_term(
        &self,
        activity_type: &str,
        _location: Option<&geo::Geometry<f32>>,
    ) -> Result<f64, OutputPluginError> {
        match self {
            ActivityFrequencies::GlobalFrequencies {
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
