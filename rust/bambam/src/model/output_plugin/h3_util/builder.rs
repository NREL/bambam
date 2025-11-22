use std::sync::Arc;

use routee_compass::{
    app::compass::CompassComponentError,
    plugin::output::{OutputPlugin, OutputPluginBuilder},
};
use routee_compass_core::config::CompassConfigurationError;
use serde_json::Value;

pub struct H3UtilOutputPluginBuilder {}

impl OutputPluginBuilder for H3UtilOutputPluginBuilder {
    fn build(&self, parameters: &Value) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        todo!()
    }
}
