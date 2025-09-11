use routee_compass_core::model::{
    label::{label_model_error::LabelModelError, label_model_service::LabelModelService, Label},
    network::VertexId,
    state::{StateModel, StateVariable},
};

pub struct MultimodalLabelService {}

impl LabelModelService for MultimodalLabelService {
    fn build(
        &self,
        query: &serde_json::Value,
        state_model: std::sync::Arc<routee_compass_core::model::state::StateModel>,
    ) -> Result<
        std::sync::Arc<dyn routee_compass_core::model::label::LabelModel>,
        routee_compass_core::model::label::label_model_error::LabelModelError,
    > {
        todo!()
    }
}
