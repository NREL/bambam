use super::{link_speed_engine::LinkSpeedEngine, link_speed_service::LinkSpeedService};
use itertools::Itertools;
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub struct LinkSpeedBuilder {
    models: HashMap<String, Rc<dyn TraversalModelBuilder>>,
}

impl LinkSpeedBuilder {
    pub fn new(models: HashMap<String, Rc<dyn TraversalModelBuilder>>) -> LinkSpeedBuilder {
        LinkSpeedBuilder { models }
    }
}

impl TraversalModelBuilder for LinkSpeedBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let underlying_type = parameters
            .get_config_string(&"underlying_type", &"traversal model")
            .map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failure reading underlying_type of link speed traversal model: {}",
                    e
                ))
            })?;
        let underlying_builder = self.models.get(&underlying_type).ok_or_else(|| {
            let options = self.models.keys().join(",");
            let msg = format!(
                "unknown underlying model type '{}' for link speed model, must be one of {}",
                underlying_type, options
            );
            TraversalModelError::BuildError(msg)
        })?;
        let underlying_service = underlying_builder.build(parameters)?;
        let engine = LinkSpeedEngine::new(parameters, underlying_service)?;
        let service = LinkSpeedService::new(Arc::new(engine))?;
        Ok(Arc::new(service))
    }
}
