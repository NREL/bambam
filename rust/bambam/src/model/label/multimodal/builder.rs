use routee_compass_core::model::{
    label::{
        label_model_builder::LabelModelBuilder, label_model_error::LabelModelError,
        label_model_service::LabelModelService, Label,
    },
    network::VertexId,
    state::{StateModel, StateVariable},
};

pub struct MultimodalLabelBuilder {}

impl LabelModelBuilder for MultimodalLabelBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<std::sync::Arc<dyn LabelModelService>, LabelModelError> {
        todo!()
    }
}
