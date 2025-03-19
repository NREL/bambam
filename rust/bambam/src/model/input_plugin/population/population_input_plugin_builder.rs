use super::population_input_plugin::PopulationInputPlugin;
use routee_compass::plugin::input::{InputPlugin, InputPluginBuilder};
use routee_compass_core::config::CompassConfigurationError;
use std::sync::Arc;

pub struct PopulationInputPluginBuilder {}

impl InputPluginBuilder for PopulationInputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn InputPlugin>, CompassConfigurationError> {
        Ok(Arc::new(PopulationInputPlugin {}))
    }
}
