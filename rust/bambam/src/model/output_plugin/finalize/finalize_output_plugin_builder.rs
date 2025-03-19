use super::finalize_output_plugin::FinalizeOutputPlugin;
use routee_compass::{app::compass::CompassComponentError, plugin::output::OutputPluginBuilder};
use routee_compass_core::config::{CompassConfigurationError, ConfigJsonExtensions};
use std::sync::Arc;

pub struct FinalizeOutputPluginBuilder {}

impl OutputPluginBuilder for FinalizeOutputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn routee_compass::plugin::output::OutputPlugin>, CompassComponentError> {
        let id_field_name = parameters.get_config_string(&"id_field_name", &"finalize")?;

        Ok(Arc::new(FinalizeOutputPlugin { id_field_name }))
    }
}
