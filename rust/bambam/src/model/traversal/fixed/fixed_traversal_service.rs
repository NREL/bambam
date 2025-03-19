use super::fixed_traversal_engine::FixedTraversalEngine;
use super::fixed_traversal_model::FixedTraversalModel;
use routee_compass_core::model::traversal::TraversalModel;
use routee_compass_core::model::traversal::TraversalModelError;
use routee_compass_core::model::traversal::TraversalModelService;
use std::sync::Arc;

pub struct FixedTraversalService {
    pub engine: Arc<FixedTraversalEngine>,
}

impl FixedTraversalService {
    pub fn new(params: &serde_json::Value) -> Result<FixedTraversalService, TraversalModelError> {
        let engine = FixedTraversalEngine::new(params)?;
        let result = FixedTraversalService {
            engine: Arc::new(engine),
        };
        Ok(result)
    }
}

impl TraversalModelService for FixedTraversalService {
    fn build(
        &self,
        _query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let engine = self.engine.clone();
        let model = FixedTraversalModel { engine };
        Ok(Arc::new(model))
    }
}
