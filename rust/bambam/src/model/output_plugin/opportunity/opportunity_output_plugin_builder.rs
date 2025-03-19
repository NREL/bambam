use super::{
    opportunity_format::OpportunityCollectFormat, opportunity_model_config::OpportunityModelConfig,
    opportunity_output_plugin::OpportunityOutputPlugin,
};
use routee_compass::{
    app::compass::CompassComponentError,
    plugin::{
        output::{OutputPlugin, OutputPluginBuilder},
        PluginError,
    },
};
use routee_compass_core::config::{CompassConfigurationError, ConfigJsonExtensions};
use std::sync::Arc;

/// RouteE Compass OutputPluginBuilder for appending opportunity counts to a bambam
/// search result row.
pub struct OpportunityOutputPluginBuilder {}

impl OutputPluginBuilder for OpportunityOutputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let config_json = parameters.get("model").ok_or_else(|| {
            CompassConfigurationError::ExpectedFieldForComponent(
                String::from("model"),
                String::from("opportunity"),
            )
        })?;

        let config: OpportunityModelConfig = serde_json::from_value(config_json.to_owned())
            .map_err(CompassConfigurationError::SerdeDeserializationError)?;
        let output_format: OpportunityCollectFormat = parameters.get_config_serde(
            &String::from("collect_format"),
            &String::from("opportunities"),
        )?;
        let plugin =
            OpportunityOutputPlugin::new(&config, output_format).map_err(PluginError::from)?;
        Ok(Arc::new(plugin))
    }
}
