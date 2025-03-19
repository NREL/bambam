use super::error_handler_plugin::ErrorHandlerPlugin;
use routee_compass::{
    app::compass::CompassComponentError,
    plugin::output::{OutputPlugin, OutputPluginBuilder},
};
use routee_compass_core::config::CompassConfigurationError;
use std::sync::Arc;

pub struct ErrorHandlerBuilder {}

impl OutputPluginBuilder for ErrorHandlerBuilder {
    fn build(
        &self,
        _parameters: &serde_json::Value,
    ) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let plugin = ErrorHandlerPlugin {};
        Ok(Arc::new(plugin))
    }
}
