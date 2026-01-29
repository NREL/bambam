use std::sync::Arc;

use super::{Flex2Config, Flex2Engine, Flex2Service};

use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};

pub struct Flex2Builder {}

impl TraversalModelBuilder for Flex2Builder {
    fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: Flex2Config = serde_json::from_value(config.clone()).map_err(|e| {
            let msg = format!("failure reading config for Flex2 builder: {e}");
            TraversalModelError::BuildError(msg)
        })?;
        let engine = Flex2Engine::try_from(config).map_err(|e| {
            let msg = format!("failure building engine from config for Flex2 builder: {e}");
            TraversalModelError::BuildError(msg)
        })?;
        let service = Flex2Service::new(engine);
        Ok(Arc::new(service))
    }
}
