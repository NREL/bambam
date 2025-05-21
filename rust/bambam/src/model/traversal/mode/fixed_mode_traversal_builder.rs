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

/// convenience traversal modeling implementation for fixed traversal modes, such as bike or walk,
/// where the same speed is used for each road network link.
pub struct FixedModeTraversalBuilder {}

impl TraversalModelBuilder for FixedModeTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let services = vec![
            DistanceTraversalBuilder {}.build(parameters)?,
            FixedSpeedBuilder {}.build(parameters)?,
            TimeTraversalBuilder {}.build(parameters)?,
        ];
        let service = CombinedTraversalService::new(services);
        Ok(Arc::new(service))
    }
}
