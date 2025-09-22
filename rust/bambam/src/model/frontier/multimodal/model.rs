use std::sync::Arc;

use crate::model::frontier::multimodal::MultimodalConstraintConfig;
use crate::model::state::{MultimodalMapping, MultimodalStateMapping};
use crate::model::{
    frontier::multimodal::MultimodalConstraint, state::multimodal_state_ops as state_ops,
};
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError},
    network::Edge,
    state::{StateModel, StateVariable},
};

pub struct MultimodalFrontierModel {
    /// maps EdgeListIds to Modes
    mode_to_edge_list: Arc<MultimodalMapping<String, usize>>,
    /// maps state variables to Modes
    mode_to_state: Arc<MultimodalStateMapping>,
    /// logic of frontier validation
    constraints: Arc<Vec<MultimodalConstraint>>,
    /// maximum number of trip legs allowed in a trip
    max_trip_legs: u64,
}

impl MultimodalFrontierModel {
    pub fn new(
        max_trip_legs: u64,
        mode_to_state: Arc<MultimodalStateMapping>,
        mode_to_edge_list: Arc<MultimodalMapping<String, usize>>,
        constraints: Arc<Vec<MultimodalConstraint>>,
    ) -> Self {
        Self {
            max_trip_legs,
            mode_to_state,
            mode_to_edge_list,
            constraints,
        }
    }

    /// builds a new [`MultimodalFrontierModel`] from its data dependencies only.
    /// used in synchronous contexts like scripting or testing.
    pub fn new_local(
        max_trip_legs: u64,
        modes: &[&str],
        edge_lists: &[&str],
        constraints: Vec<MultimodalConstraint>,
    ) -> Result<Self, FrontierModelError> {
        let mode_to_state =
            MultimodalMapping::new(&modes.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                .map_err(|e| {
                FrontierModelError::BuildError(format!(
                    "while building MultimodalFrontierModel, failure constructing mode mapping: {e}"
                ))
            })?;

        let mode_to_edge_list = MultimodalMapping::new(
            &edge_lists
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .map_err(|e| {
            FrontierModelError::BuildError(format!(
                "while building MultimodalFrontierModel, failure constructing mode mapping: {e}"
            ))
        })?;

        let mmm = Self::new(
            max_trip_legs,
            Arc::new(mode_to_state),
            Arc::new(mode_to_edge_list),
            Arc::new(constraints),
        );
        Ok(mmm)
    }
}

impl FrontierModel for MultimodalFrontierModel {
    /// confirms that, upon reaching this edge,
    ///   - we have not exceeded any mode-specific distance, time or energy limit
    /// confirms that, if we add this edge,
    ///   - we have not exceeded max trip legs
    ///   - we have not exceeded max mode counts
    ///   - our trip still matches any exact mode sequences
    fn valid_frontier(
        &self,
        edge: &Edge,
        previous_edge: Option<&Edge>,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        for constraint in self.constraints.iter() {
            let valid = constraint.valid_frontier(
                edge,
                state,
                state_model,
                &self.mode_to_state,
                &self.mode_to_edge_list,
                self.max_trip_legs,
            )?;
            if !valid {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn valid_edge(&self, edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use crate::model::frontier::multimodal::model::MultimodalFrontierModel;

    #[test]
    fn test_valid_n_legs_empty() {
        let mfm = MultimodalFrontierModel::new_local(1, &[], &[], vec![]).expect("test failed");
    }

    #[test]
    fn test_valid_n_legs() {}

    #[test]
    fn test_invalid_n_legs() {}

    #[test]
    fn test_valid_mode_counts() {}

    #[test]
    fn test_invalid_mode_counts() {}
}
