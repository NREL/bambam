use std::sync::Arc;

use routee_compass::{
    app::compass::CompassComponentError,
    plugin::{
        output::{OutputPlugin, OutputPluginBuilder, OutputPluginError},
        PluginError,
    },
};
use routee_compass_core::config::CompassConfigurationError;
use serde_json::Value;

use crate::model::output_plugin::h3_util::{H3Util, H3UtilInputPlugin, H3UtilOutputPluginConfig};

pub struct H3UtilOutputPluginBuilder {}

impl OutputPluginBuilder for H3UtilOutputPluginBuilder {
    fn build(&self, parameters: &Value) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let config: H3UtilOutputPluginConfig =
            serde_json::from_value(parameters.clone()).map_err(|e| {
                PluginError::OutputPluginFailed {
                    source: OutputPluginError::BuildFailed(format!(
                        "failed reading h3 util configuration: {e}"
                    )),
                }
            })?;
        let util = H3Util::try_from(&config)?;
        let plugin = H3UtilInputPlugin::new(util);
        Ok(Arc::new(plugin))
    }
}
