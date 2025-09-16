use routee_compass_core::model::{
    label::{label_model_error::LabelModelError, Label, LabelModel},
    network::VertexId,
    state::{StateModel, StateVariable},
};

use crate::model::state::MultimodalMapping;

pub struct MultimodalLabelModel {
    mapping: MultimodalMapping<String, i64>,
    max_trip_legs: usize,
}

impl MultimodalLabelModel {
    pub fn new(
        mapping: MultimodalMapping<String, i64>,
        max_trip_legs: usize,
    ) -> MultimodalLabelModel {
        MultimodalLabelModel {
            mapping,
            max_trip_legs,
        }
    }
}

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
