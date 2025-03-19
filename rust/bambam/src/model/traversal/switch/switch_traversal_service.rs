use itertools::Itertools;
use routee_compass_core::{
    config::{CompassConfigurationError, ConfigJsonExtensions},
    model::traversal::{TraversalModel, TraversalModelError, TraversalModelService},
};
use std::{collections::HashMap, sync::Arc};

pub struct SwitchTraversalService {
    pub services: HashMap<String, Arc<dyn TraversalModelService>>,
}

impl TraversalModelService for SwitchTraversalService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let mode = query
            .get_config_string(&String::from("mode"), &String::from("query"))
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        match self.services.get(&mode) {
            None => {
                let err_config = CompassConfigurationError::UnknownModelNameForComponent(
                    mode.clone(),
                    String::from("traversal model"),
                    self.services.keys().join(", "),
                );
                let err = TraversalModelError::BuildError(err_config.to_string());
                Err(err)
            }
            Some(service) => service.build(query),
        }
    }
}
