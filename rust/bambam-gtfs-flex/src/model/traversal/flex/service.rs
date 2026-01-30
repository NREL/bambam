use std::sync::Arc;

use super::{GtfsFlexEngine, GtfsFlexModel, GtfsFlexParams};

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};

pub struct GtfsFlexService {
    engine: Arc<GtfsFlexEngine>,
}

impl GtfsFlexService {
    pub fn new(engine: GtfsFlexEngine) -> Self {
        Self {
            engine: Arc::new(engine),
        }
    }
}

impl TraversalModelService for GtfsFlexService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let params: GtfsFlexParams = serde_json::from_value(query.clone()).map_err(|e| {
            let msg = format!("failure reading params for GtfsFlex service: {e}");
            TraversalModelError::BuildError(msg)
        })?;
        let model = GtfsFlexModel::new(self.engine.clone(), params);
        Ok(Arc::new(model))
    }
}
