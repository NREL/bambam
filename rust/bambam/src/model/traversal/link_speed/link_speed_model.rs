use super::super::super::bambam_state_ops;
use super::link_speed_engine::LinkSpeedEngine;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{StateFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use std::sync::Arc;

pub struct LinkSpeedModel {
    pub engine: Arc<LinkSpeedEngine>,
    pub underlying: Arc<dyn TraversalModel>,
}

impl TraversalModel for LinkSpeedModel {
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
        let (src, edge, dst) = trajectory;
        // let traversal_speed = get_speed(&self.engine.speed_table, edge.edge_id)?;
        // let edge_speed = (&traversal_speed, &self.engine.speed_unit);

        bambam_state_ops::assign_departure_delay(
            src,
            edge,
            state,
            state_model,
            &self.engine.departure_delay,
        )?;
        self.underlying
            .traverse_edge(trajectory, state, state_model)?;
        bambam_state_ops::assign_arrival_delay(
            dst,
            state,
            state_model,
            &self.engine.arrival_delay,
        )?;

        // mep_state_ops::default_mep_traversal(
        //     trajectory,
        //     edge_speed,
        //     state,
        //     state_model,
        //     &self.engine.departure_delay,
        //     &self.engine.arrival_delay,
        // )?;

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
