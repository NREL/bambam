use std::sync::Arc;

use crate::model::traversal::multimodal::MultimodalTraversalModel;

use super::MultimodalTraversalConfig;
use itertools::Itertools;
use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};
use serde_json::Value;

pub struct MultimodalTraversalService {
    pub config: Arc<MultimodalTraversalConfig>,
}

impl MultimodalTraversalService {
    pub fn new(config: Arc<MultimodalTraversalConfig>) -> MultimodalTraversalService {
        MultimodalTraversalService { config }
    }
}

impl TraversalModelService for MultimodalTraversalService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let mode_json = query.get("mode").ok_or_else(|| {
            TraversalModelError::BuildError(String::from("incoming query is missing 'mode' field"))
        })?;
        validate_is_string(mode_json)?;

        let mode = mode_json.as_str().ok_or_else(|| {
            TraversalModelError::BuildError(format!(
                "unknown 'mode': {}",
                serde_json::to_string(mode_json).unwrap_or_default()
            ))
        })?;
        let feature_dependencies = self.config.modes.get(mode).ok_or_else(|| {
            TraversalModelError::BuildError(format!(
                "mode '{}' not found, must be one of [{}]",
                mode,
                self.config.modes.keys().join(", ")
            ))
        })?;
        let model = Arc::new(MultimodalTraversalModel::new(feature_dependencies.to_vec()));
        Ok(model)
    }
}

fn validate_is_string(value: &Value) -> Result<(), TraversalModelError> {
    match value {
        serde_json::Value::Null => Err(TraversalModelError::BuildError(String::from(
            "incoming query 'mode' is Null but must be a string",
        ))),
        serde_json::Value::Bool(_) => Err(TraversalModelError::BuildError(String::from(
            "incoming query 'mode' is Bool but must be a string",
        ))),
        serde_json::Value::Number(number) => Err(TraversalModelError::BuildError(String::from(
            "incoming query 'mode' is Number but must be a string",
        ))),
        serde_json::Value::Array(values) => Err(TraversalModelError::BuildError(String::from(
            "incoming query 'mode' is Array but must be a string",
        ))),
        serde_json::Value::Object(map) => Err(TraversalModelError::BuildError(String::from(
            "incoming query 'mode' is Object but must be a string",
        ))),
        serde_json::Value::String(_) => Ok(()),
    }
}
