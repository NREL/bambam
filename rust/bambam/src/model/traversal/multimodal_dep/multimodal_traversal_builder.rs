use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

use crate::model::traversal::multimodal_dep::{
    multimodal_traversal_service::MultimodalTraversalService, MultimodalTraversalConfig,
};

pub struct MultimodalTraversalBuilder {}

impl TraversalModelBuilder for MultimodalTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: MultimodalTraversalConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failed to read multimodal traversal configuration object, line {}, col {}: {e}", e.line(), e.column()
                ))
            })?;
        let service = MultimodalTraversalService::new(Arc::new(config));
        Ok(Arc::new(service))
    }
}
