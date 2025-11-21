use std::sync::Arc;

use routee_compass::plugin::input::{InputPlugin, InputPluginBuilder};
use routee_compass_core::config::CompassConfigurationError;
use serde_json::Value;

pub struct H3UtilInputPluginBuilder {}

impl InputPluginBuilder for H3UtilInputPluginBuilder {
    fn build(&self, parameters: &Value) -> Result<Arc<dyn InputPlugin>, CompassConfigurationError> {
        todo!()
    }
}
