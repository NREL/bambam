use super::link_speed_engine::LinkSpeedEngine;
use super::link_speed_model::LinkSpeedModel;
use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

pub struct LinkSpeedService {
    engine: Arc<LinkSpeedEngine>,
}

impl LinkSpeedService {
    pub fn new(engine: Arc<LinkSpeedEngine>) -> Result<LinkSpeedService, TraversalModelError> {
        Ok(LinkSpeedService { engine })
    }
}

impl TraversalModelService for LinkSpeedService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let engine = self.engine.clone();
        let underlying = self.engine.underlying_service.build(query)?;
        let model = LinkSpeedModel { engine, underlying };
        Ok(Arc::new(model))
    }
}
