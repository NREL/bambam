use routee_compass_core::model::{
    label::{label_model_error::LabelModelError, Label, LabelModel},
    network::VertexId,
    state::{StateModel, StateVariable},
};

pub struct MultimodalLabelModel {}

impl LabelModel for MultimodalLabelModel {
    fn label_from_state(
        &self,
        vertex_id: VertexId,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<Label, LabelModelError> {
        // building a `LabelEnum::VertexWithIntStateVec`?
        todo!()
    }
}
