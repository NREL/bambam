use routee_compass_core::{
    algorithm::search::SearchTree,
    model::{
        network::{Edge, Vertex},
        state::{InputFeature, StateModel, StateVariable, StateVariableConfig},
        traversal::{TraversalModel, TraversalModelError},
    },
};

/// models travel modes that are described in GBFS file sources.
pub struct GbfsTraversalModel {}

impl TraversalModel for GbfsTraversalModel {
    fn name(&self) -> String {
        "GbfsTraversalModel".to_string()
    }

    fn input_features(&self) -> Vec<InputFeature> {
        todo!()
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        todo!()
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        tree: &SearchTree,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        tree: &SearchTree,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        // this can be skipped if we aren't trying to use A*.
        Ok(())
    }
}
