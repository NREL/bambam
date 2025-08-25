use std::sync::Arc;

use crate::model::traversal::multimodal::{FeatureDependency, MultimodalTraversalModel};

use super::MultimodalTraversalConfig;
use itertools::Itertools;
use routee_compass_core::model::{
    state::StateFeature,
    traversal::{TraversalModel, TraversalModelError, TraversalModelService},
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
        let dependency_configurations = self.config.dependencies.get(mode).ok_or_else(|| {
            TraversalModelError::BuildError(format!(
                "mode '{}' not found, must be one of [{}]",
                mode,
                self.config.dependencies.keys().join(", ")
            ))
        })?;

        // get all output features here but dedup by field name. all StateFeatures with the same key must have
        // matching definitions in order to accept it.
        let feature_dependencies = dependency_configurations
            .iter()
            .map(|conf| FeatureDependency::new(conf, &self.config.output_features))
            .try_collect()?;
        let output_features = self
            .config
            .output_features
            .iter()
            .map(|(n, f)| (n.clone(), *f))
            .collect_vec();

        let model = Arc::new(MultimodalTraversalModel::new(
            feature_dependencies,
            output_features,
        ));
        Ok(model)
    }
}

/// helper that is an improvement over as_str().ok_or_else(|| ..) for getting a Value
/// as a string (included "wrong" Value variant in error message).
fn validate_is_string(value: &Value) -> Result<(), TraversalModelError> {
    match value {
        serde_json::Value::String(_) => Ok(()),
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
    }
}
