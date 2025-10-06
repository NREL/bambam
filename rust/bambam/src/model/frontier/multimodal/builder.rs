use std::sync::Arc;

use routee_compass_core::model::frontier::{
    FrontierModelBuilder, FrontierModelError, FrontierModelService,
};

use crate::model::frontier::multimodal::{MultimodalFrontierConfig, MultimodalFrontierService};

pub struct MultimodalFrontierBuilder {}

impl FrontierModelBuilder for MultimodalFrontierBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        let config: MultimodalFrontierConfig =
            serde_json::from_value(parameters.clone()).map_err(|e| {
                FrontierModelError::BuildError(format!(
                    "while reading multimodal frontier model configuration: {e}"
                ))
            })?;
        let service = MultimodalFrontierService::new(config)?;
        Ok(Arc::new(service))
    }
}
