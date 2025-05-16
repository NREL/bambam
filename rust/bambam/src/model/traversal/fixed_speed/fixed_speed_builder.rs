use super::fixed_speed_model::FixedSpeedModel;
use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

pub struct FixedSpeedBuilder {}

impl TraversalModelBuilder for FixedSpeedBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let service: FixedSpeedModel = serde_json::from_value(parameters.clone()).map_err(|e| {
            TraversalModelError::BuildError(format!(
                "failure reading fixed traversal configuration: {}",
                e,
            ))
        })?;

        Ok(Arc::new(service))
    }
}
