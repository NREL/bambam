use super::fixed_traversal_engine::FixedTraversalEngine;
use crate::model::bambam_state_ops;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{StateFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use std::sync::Arc;

/// Traversal Model for a fixed-speed modes like Walk and Bike
pub struct FixedTraversalModel {
    pub engine: Arc<FixedTraversalEngine>,
}

impl TraversalModel for FixedTraversalModel {
    fn state_features(&self) -> Vec<(String, StateFeature)> {
        bambam_state_ops::default_state_features()
    }

    /// computes the cost of traversing a link for some fixed-speed
    /// travel mode.
    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let edge_speed = (&self.engine.speed, &self.engine.speed_unit);
        bambam_state_ops::default_mep_traversal(
            trajectory,
            edge_speed,
            state,
            state_model,
            &self.engine.departure_delay,
            &self.engine.arrival_delay,
        )?;

        Ok(())
    }

    /// no cost estimates for isochrone searches
    fn estimate_traversal(
        &self,
        _od: (&Vertex, &Vertex),
        _state: &mut Vec<StateVariable>,
        _state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        Ok(())
    }
}
