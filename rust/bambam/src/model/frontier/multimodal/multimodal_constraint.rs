use crate::model::frontier::multimodal::multimodal_frontier_ops as ops;
use crate::model::state::{
    multimodal_state_ops as state_ops, MultimodalMapping, MultimodalStateMapping,
};
use routee_compass_core::model::{
    frontier::FrontierModelError,
    network::Edge,
    state::{StateModel, StateVariable},
    unit::TimeUnit,
};
use std::collections::{HashMap, HashSet};
use uom::si::f64::Time;

/// types of constraints to limit exponential search expansion in multimodal scenarios.
pub enum MultimodalConstraint {
    AllowedModes(HashSet<String>),
    ModeCounts(HashMap<String, usize>),
    MaxTripLegs(usize),
    MaxTime(HashMap<String, Time>),
}

impl MultimodalConstraint {
    /// validates an edge for traversal in a multimodal traversal
    pub fn valid_frontier(
        &self,
        edge: &Edge,
        state: &[StateVariable],
        state_model: &StateModel,
        mode_to_state: &MultimodalStateMapping,
        mode_to_edge_list: &MultimodalMapping<String, usize>,
        max_trip_legs: u64,
    ) -> Result<bool, FrontierModelError> {
        match self {
            MultimodalConstraint::AllowedModes(items) => {
                let edge_mode = ops::get_edge_list_mode(edge, mode_to_edge_list)?;
                Ok(items.contains(edge_mode))
            }
            MultimodalConstraint::ModeCounts(limits) => {
                let edge_mode = ops::get_edge_list_mode(edge, mode_to_edge_list)?;
                let mut counts =
                    ops::get_mode_counts(state, state_model, max_trip_legs, mode_to_state)?;
                counts
                    .entry(edge_mode.clone())
                    .and_modify(|cnt| *cnt += 1)
                    .or_insert(1);
                Ok(ops::valid_mode_counts(&counts, limits))
            }
            MultimodalConstraint::MaxTripLegs(max_legs) => {
                let n_legs = state_ops::get_n_legs(state, state_model).map_err(|e| {
                    FrontierModelError::FrontierModelError(
                        (format!("while getting number of trip legs for this trip: {e}")),
                    )
                })?;
                Ok(n_legs <= *max_legs)
            }
            MultimodalConstraint::MaxTime(limits) => {
                ops::valid_mode_time(state, state_model, limits)
            }
        }
    }
}
