use std::sync::Arc;

use routee_compass_core::model::{
    label::{
        label_model_builder::LabelModelBuilder, label_model_error::LabelModelError,
        label_model_service::LabelModelService, Label,
    },
    network::VertexId,
    state::{StateModel, StateVariable},
};

use crate::model::label::multimodal::{MultimodalLabelConfig, MultimodalLabelService};

pub struct MultimodalLabelBuilder {}

impl LabelModelBuilder for MultimodalLabelBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn LabelModelService>, LabelModelError> {
        let conf: MultimodalLabelConfig =
            serde_json::from_value(parameters.clone()).map_err(|e| {
                LabelModelError::LabelModelError(format!(
                    "failure reading multimodal label model config: {e}"
                ))
            })?;
        let service = MultimodalLabelService::new(conf);
        Ok(Arc::new(service))
    }
}
