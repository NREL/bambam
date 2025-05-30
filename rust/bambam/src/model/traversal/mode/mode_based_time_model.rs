use super::ModeBasedTimeModelConfiguration;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{InputFeature, OutputFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError, TraversalModelService},
};
use std::sync::Arc;

pub struct ModeBasedTimeModel {
    pub config: Arc<ModeBasedTimeModelConfiguration>,
}

impl TraversalModelService for ModeBasedTimeModel {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        todo!()
    }
}

impl TraversalModel for ModeBasedTimeModel {
    fn input_features(&self) -> Vec<(String, InputFeature)> {
        todo!()
    }

    fn output_features(&self) -> Vec<(String, OutputFeature)> {
        todo!()
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }
}
