use std::sync::Arc;

use routee_compass_core::model::{
    label::{
        label_model_error::{self, LabelModelError},
        label_model_service::LabelModelService,
        Label, LabelModel,
    },
    network::VertexId,
    state::{StateModel, StateVariable},
};

pub struct MultimodalLabelService {}

impl LabelModelService for MultimodalLabelService {
    fn build(
        &self,
        query: &serde_json::Value,
        state_model: Arc<StateModel>,
    ) -> Result<std::sync::Arc<dyn LabelModel>, label_model_error::LabelModelError> {
        todo!()
    }
}
