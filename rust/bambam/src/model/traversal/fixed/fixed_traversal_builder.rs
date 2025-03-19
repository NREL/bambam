use super::fixed_traversal_service::FixedTraversalService;
use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

pub struct FixedTraversalBuilder {}

impl TraversalModelBuilder for FixedTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let service = FixedTraversalService::new(parameters)?;
        Ok(Arc::new(service))
    }
}
