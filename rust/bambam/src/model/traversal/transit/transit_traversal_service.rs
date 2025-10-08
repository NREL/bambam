use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};

use crate::model::traversal::transit::transit_traversal_engine::TransitTraversalEngine;

pub struct TransitTraversalService {
    engine: Arc<TransitTraversalEngine>,
}

impl TransitTraversalService {
    pub fn new(engine: Arc<TransitTraversalEngine>) -> Self {
        Self {
            engine: engine.clone(),
        }
    }
}

impl TraversalModelService for TransitTraversalService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<std::sync::Arc<dyn TraversalModel>, TraversalModelError> {
        todo!()
    }
}
