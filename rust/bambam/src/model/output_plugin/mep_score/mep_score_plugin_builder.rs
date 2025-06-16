use crate::model::output_plugin::mep_score::{
    activity_parameters::ActivityParameters, modal_intensity_model::ModalIntensityModel,
    ActivityParametersConfig, ModalIntensityConfig,
};
use routee_compass::{
    app::compass::CompassComponentError,
    plugin::output::{OutputPlugin, OutputPluginBuilder},
};
use routee_compass_core::config::ConfigJsonExtensions;
use std::sync::Arc;

use super::mep_score_output_plugin::MepScoreOutputPlugin;

pub struct MepScoreOutputPluginBuilder {}

impl OutputPluginBuilder for MepScoreOutputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let parent_key = String::from("mep_score");

        let modal_intensity_config: ModalIntensityConfig =
            parameters.get_config_serde(&String::from("modal_intensity_values"), &parent_key)?;
        let modal_intensity_values = ModalIntensityModel::try_from(&modal_intensity_config)?;
        let activity_parameters_config: ActivityParametersConfig =
            parameters.get_config_serde(&String::from("activity_parameters"), &parent_key)?;
        let activity_parameters = ActivityParameters::try_from(&activity_parameters_config)?;
        let plugin = MepScoreOutputPlugin {
            modal_intensity_values,
            activity_parameters,
        };
        Ok(Arc::new(plugin))
    }
}
