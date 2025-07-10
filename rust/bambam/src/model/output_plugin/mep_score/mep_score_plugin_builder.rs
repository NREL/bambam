use super::mep_score_plugin::MepScorePlugin;
use crate::model::output_plugin::mep_score::{
    activity_frequencies::ActivityFrequencies, modal_intensity_model::ModalIntensityModel,
    ActivityFrequenciesConfig, MepScorePluginConfig, ModalIntensityConfig, WeightingFactors,
};
use routee_compass::{
    app::compass::CompassComponentError,
    plugin::output::{OutputPlugin, OutputPluginBuilder},
};
use routee_compass_core::config::{CompassConfigurationError, ConfigJsonExtensions};
use std::sync::Arc;

pub struct MepScoreOutputPluginBuilder {}

impl OutputPluginBuilder for MepScoreOutputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let config: MepScorePluginConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| CompassConfigurationError::SerdeDeserializationError(e))?;
        let plugin = MepScorePlugin::try_from(&config)?;
        Ok(Arc::new(plugin))
    }
}
