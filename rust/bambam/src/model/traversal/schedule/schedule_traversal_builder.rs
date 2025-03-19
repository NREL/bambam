use super::schedule_traversal_service::ScheduleTraversalService;
use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

pub struct ScheduleTraversalBuilder {}

impl TraversalModelBuilder for ScheduleTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let service = ScheduleTraversalService::new(parameters)?;
        Ok(Arc::new(service))
    }
}
