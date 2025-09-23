use std::collections::HashMap;

use crate::model::state::{
    multimodal_state_ops as state_ops, MultimodalMapping, MultimodalStateMapping,
};
use itertools::Itertools;
use routee_compass_core::model::{
    frontier::FrontierModelError,
    network::Edge,
    state::{StateModel, StateVariable},
};
use uom::si::f64::Time;

/// helper to get the edge list mode for an edge
pub fn get_edge_list_mode<'a>(
    edge: &Edge,
    mode_to_edge_list: &'a MultimodalMapping<String, usize>,
) -> Result<&'a String, FrontierModelError> {
    mode_to_edge_list
        .get_categorical(edge.edge_list_id.0)
        .map_err(|e| {
            FrontierModelError::FrontierModelError(
                "failure getting edge list mode via edge list mode mapping".to_string(),
            )
        })?
        .ok_or_else(|| {
            FrontierModelError::FrontierModelError(format!(
                "multimodal frontier model has no mode for edge list {}",
                edge.edge_list_id
            ))
        })
}

/// count how many times a travel mode is used during a trip by each trip leg.
pub fn get_mode_counts(
    state: &[StateVariable],
    state_model: &StateModel,
    max_trip_legs: u64,
    mode_to_state: &MultimodalStateMapping,
) -> Result<HashMap<String, usize>, FrontierModelError> {
    let modes = state_ops::get_mode_sequence(state, state_model, max_trip_legs, mode_to_state)
        .map_err(|e| {
            FrontierModelError::FrontierModelError(
                (format!("while getting mode counts for this trip: {e}")),
            )
        })?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for mode in modes.into_iter() {
        counts.entry(mode).and_modify(|cnt| *cnt += 1).or_insert(1);
    }
    Ok(counts)
}

/// validates the observed number of mode counts against the provided limits
pub fn valid_mode_counts(counts: &HashMap<String, usize>, limits: &HashMap<String, usize>) -> bool {
    for (mode, observed) in counts.iter() {
        match limits.get(mode) {
            Some(limit) if observed > limit => return false,
            None => return false,
            _ => { /* no op */ }
        }
    }
    true
}

pub fn valid_mode_time(
    state: &[StateVariable],
    state_model: &StateModel,
    limits: &HashMap<String, Time>,
) -> Result<bool, FrontierModelError> {
    for (mode, limit) in limits.iter() {
        let mode_time = state_ops::get_mode_time(state, mode, state_model).map_err(|e| {
            FrontierModelError::FrontierModelError(
                (format!("while validating mode time limits for '{mode}': {e}")),
            )
        })?;
        if &mode_time > limit {
            return Ok(false);
        }
    }
    Ok(true)
}
