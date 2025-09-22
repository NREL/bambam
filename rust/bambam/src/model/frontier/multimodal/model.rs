use crate::model::state::MultimodalMapping;
use crate::model::{
    frontier::multimodal::MultimodalConstraint, state::multimodal_state_ops as state_ops,
};
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError},
    network::Edge,
    state::{StateModel, StateVariable},
};

pub struct MultimodalFrontierModel {
    edge_list_mapping: MultimodalMapping<String, usize>,
    mode_mapping: MultimodalMapping<String, i64>,
    constraints: Vec<MultimodalConstraint>,
    max_trip_legs: u64,
}

impl FrontierModel for MultimodalFrontierModel {
    fn valid_frontier(
        &self,
        edge: &Edge,
        previous_edge: Option<&Edge>,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        let edge_mode = self
            .edge_list_mapping
            .get_categorical(edge.edge_list_id.0)
            .map_err(|e| {
                FrontierModelError::FrontierModelError(format!(
                    "failure getting edge list mode via edge list mode mapping"
                ))
            })?
            .ok_or_else(|| {
                FrontierModelError::FrontierModelError(format!(
                    "multimodal frontier model has no mode for edge list {}",
                    edge.edge_list_id
                ))
            })?;
        let active_leg_opt = state_ops::get_active_leg_idx(state, state_model).map_err(|e| {
            FrontierModelError::FrontierModelError(format!(
                "during multimodal frontier model, failed getting active leg due to: {e}"
            ))
        })?;
        let leg_idx = match active_leg_opt {
            Some(idx) => idx,
            None => {
                todo!("test constraints, not trip history yet")
            }
        };
        let current_mode = state_ops::get_existing_leg_mode(
            state,
            leg_idx,
            state_model,
            self.max_trip_legs,
            &self.mode_mapping,
        )
        .map_err(|e| {
            FrontierModelError::FrontierModelError(format!(
                "state vector with current leg index {leg_idx} has no existing leg mode"
            ))
        })?;

        todo!("test constraints")
    }

    fn valid_edge(&self, edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}
