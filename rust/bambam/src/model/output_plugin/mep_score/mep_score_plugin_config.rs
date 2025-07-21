use crate::model::output_plugin::mep_score::{
    ActivityFrequenciesConfig, ModalIntensityConfig, WeightingFactors,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct MepScorePluginConfig {
    /// model providing the way intensity values are found
    pub modal_intensity_model: ModalIntensityConfig,
    /// factors multiplied against the found intensity values
    pub modal_weighting_factors: WeightingFactors,
    /// frequencies of engagement for activity types
    pub activity_frequencies: ActivityFrequenciesConfig,
    /// activity type used to normalize opportunity counts
    pub normalizing_activity: String,
}
