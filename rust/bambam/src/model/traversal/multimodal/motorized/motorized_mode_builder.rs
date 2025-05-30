use super::MotorizedModeConfig;
use crate::model::traversal::fixed_speed::FixedSpeedBuilder;
use routee_compass_core::model::traversal::{
    default::{
        combined::{CombinedTraversalBuilder, CombinedTraversalService},
        distance::DistanceTraversalBuilder,
        time::TimeTraversalBuilder,
    },
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

/// convenience traversal modeling implementation for motorized modes.
/// ```
pub struct MotorizedModeBuilder {}

impl TraversalModelBuilder for MotorizedModeBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config = MotorizedModeConfig::try_from(parameters)?;
        let service = config.build();
        Ok(Arc::new(service))
    }
}
