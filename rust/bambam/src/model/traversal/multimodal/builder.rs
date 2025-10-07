use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use serde_json::Value;

use crate::model::traversal::multimodal::{MultimodalTraversalConfig, MultimodalTraversalService};

pub struct MultimodalTraversalBuilder {}

impl TraversalModelBuilder for MultimodalTraversalBuilder {
    fn build(
        &self,
        parameters: &Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: MultimodalTraversalConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failure while reading multimodal traversal configuration: {e}"
                ))
            })?;
        let model = Arc::new(MultimodalTraversalService::new(config)?);
        Ok(model)
    }
}
