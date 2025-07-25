use crate::model::traversal::switch::switch_traversal_service::SwitchTraversalService;
use itertools::Itertools;
use routee_compass_core::{
    config::{CompassConfigurationError, ConfigJsonExtensions},
    model::traversal::{TraversalModelBuilder, TraversalModelError, TraversalModelService},
};
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub struct SwitchTraversalBuilder {
    models: HashMap<String, Rc<dyn TraversalModelBuilder>>,
}

impl SwitchTraversalBuilder {
    pub fn new(models: HashMap<String, Rc<dyn TraversalModelBuilder>>) -> SwitchTraversalBuilder {
        SwitchTraversalBuilder { models }
    }
}
impl TraversalModelBuilder for SwitchTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let models = parameters
            .get_config_array(&String::from("models"), &String::from("traversal"))
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let services =
            models
                .iter()
                .enumerate()
                .map(|(idx, params)| {
                    let parent_key = format!("models[{idx}]");
                    let mode =
                        params.get_config_string(&String::from("mode"),&parent_key)?;
                    let model_type = params.get_config_string(&String::from("type"), &parent_key)?;
                    let model_builder = self.models.get(&model_type).ok_or_else(|| {
                        let options = self.models.keys().join(",");
                        TraversalModelError::BuildError(format!("unknown traversal model type {model_type}, must be one of [{options}]"))
                    })?;
                    let service = model_builder.build(params)?;
                    Ok((mode, service))
                })
                .collect::<Result<
                    HashMap<String, Arc<dyn TraversalModelService>>,
                    CompassConfigurationError,
                >>().map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        log::info!(
            "loaded traversal models for '{}'",
            services.keys().join(", ")
        );
        let service = SwitchTraversalService { services };
        Ok(Arc::new(service))
    }
}
