use std::sync::Arc;

use routee_compass_core::model::access::{
    AccessModelBuilder, AccessModelError, AccessModelService,
};
use serde_json::Value;

use crate::model::{
    access::multimodal::{MultimodalAccessConfig, MultimodalAccessService},
    traversal::multimodal::{MultimodalTraversalConfig, MultimodalTraversalService},
};

pub struct MultimodalAccessBuilder {}

impl AccessModelBuilder for MultimodalAccessBuilder {
    fn build(&self, parameters: &Value) -> Result<Arc<dyn AccessModelService>, AccessModelError> {
        let config: MultimodalAccessConfig =
            serde_json::from_value(parameters.clone()).map_err(|e| {
                AccessModelError::BuildError(format!(
                    "failure while reading multimodal traversal configuration: {e}"
                ))
            })?;
        let model = Arc::new(MultimodalAccessService::new(config)?);
        Ok(model)
    }
}
