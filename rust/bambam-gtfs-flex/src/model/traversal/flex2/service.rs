use std::sync::Arc;

use super::{Flex2Engine, Flex2Params, Flex2Model};

use routee_compass_core::model::traversal::{TraversalModel, TraversalModelError, TraversalModelService};

pub struct Flex2Service {
    engine: Arc<Flex2Engine>
}

impl Flex2Service {
    pub fn new(engine: Flex2Engine) -> Self {
        Self {
            engine: Arc::new(engine)
        }
    }
}

impl TraversalModelService for Flex2Service {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let params: Flex2Params = serde_json::from_value(query.clone())
            .map_err(|e| {
                let msg = format!("failure reading params for Flex2 service: {e}");
                TraversalModelError::BuildError(msg)
            })?;
        let model = Flex2Model::new(self.engine.clone(), params);
        Ok(Arc::new(model))
    }
}
