use std::sync::Arc;

use routee_compass::{
    app::compass::CompassComponentError,
    plugin::input::{InputPlugin, InputPluginBuilder},
};
use routee_compass_core::config::CompassConfigurationError;

use crate::model::input_plugin::grid_geometry::grid_geometry_input_plugin::GridGeometryInputPlugin;

pub struct GridGeometryInputPluginBuilder {}

impl InputPluginBuilder for GridGeometryInputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn InputPlugin>, CompassConfigurationError> {
        Ok(Arc::new(GridGeometryInputPlugin {}))
    }
}
