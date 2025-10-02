use std::sync::Arc;

use crate::model::frontier::multimodal::MultimodalFrontierConstraintConfig;
use crate::model::state::{MultimodalMapping, MultimodalStateMapping};
use crate::model::{
    frontier::multimodal::MultimodalFrontierConstraint, state::multimodal_state_ops as state_ops,
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
    constraints: Arc<Vec<MultimodalFrontierConstraint>>,
    /// maximum number of trip legs allowed in a trip
    max_trip_legs: u64,
}

impl MultimodalFrontierModel {
    pub fn new(
        max_trip_legs: u64,
        mode_to_state: Arc<MultimodalStateMapping>,
        mode_to_edge_list: Arc<MultimodalMapping<String, usize>>,
        constraints: Arc<Vec<MultimodalFrontierConstraint>>,
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
        constraints: Vec<MultimodalFrontierConstraint>,
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
    use std::collections::{HashMap, HashSet};

    use itertools::Itertools;
    use routee_compass_core::model::{
        frontier::FrontierModel,
        network::Edge,
        state::{StateModel, StateVariable},
        traversal::TraversalModel,
    };
    use uom::si::f64::Length;

    use crate::model::{
        frontier::multimodal::{
            model::MultimodalFrontierModel, sequence_trie::SubSequenceTrie,
            MultimodalFrontierConstraint,
        },
        state::{multimodal_state_ops as state_ops, MultimodalStateMapping},
        traversal::multimodal::MultimodalTraversalModel,
    };

    #[test]
    fn test_valid_max_trip_legs_empty_state() {
        // testing validitity of an initial state using constraint "max trip legs = 1"
        let max_trip_legs = 1;
        let (mam, mfm, state_model, state) = test_setup(
            vec![MultimodalFrontierConstraint::MaxTripLegs(1)],
            "walk",
            &["walk", "bike"],
            &[],
            max_trip_legs,
        );

        let edge = Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_valid_n_legs() {
        // testing validitity of a state with one leg using constraint "max trip legs = 2"
        let max_trip_legs = 2;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![MultimodalFrontierConstraint::MaxTripLegs(1)],
            "walk",
            &["walk", "bike"],
            &[],
            max_trip_legs,
        );

        let edge = Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        // assign one leg to walk mode
        state_ops::set_leg_mode(&mut state, 0, "walk", &state_model, &mam.mode_to_state)
            .expect("test invariant failed");
        state_ops::increment_active_leg_idx(&mut state, &state_model, max_trip_legs)
            .expect("test invariant failed");

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_invalid_n_legs() {
        // testing validitity of a state with two legs using constraint "max trip legs = 1"
        let max_trip_legs = 2;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![MultimodalFrontierConstraint::MaxTripLegs(1)],
            "walk",
            &["walk", "bike"],
            &[],
            max_trip_legs,
        );

        // assign one leg to walk mode
        let edge = Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));
        inject_trip_legs(
            &["walk", "bike"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(!is_valid);
    }

    #[test]
    fn test_valid_mode_counts() {
        // testing validitity of traversing a "walk" edge using state with "walk", "drive", "walk" sequence.
        // our constraint is walk<=2, drive<=1. since this new edge has walk-mode, it will not increase the
        // number of trip legs, so it should be valid.
        let max_trip_legs = 5;
        let mode_constraint = MultimodalFrontierConstraint::ModeCounts(HashMap::from([
            ("walk".to_string(), 2),
            ("drive".to_string(), 1),
        ]));
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a walk-mode edge list
        let edge = Edge::new(1, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "drive", "walk"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_invalid_mode_counts() {
        // testing validitity of traversing a "drive" edge using state with "walk", "drive", "walk" sequence.
        // our constraint is walk<=2, drive<=1. since this new edge has drive-mode, it will increase the
        // number of trip legs, so it should be invalid.
        let max_trip_legs = 5;
        let mode_constraint = MultimodalFrontierConstraint::ModeCounts(HashMap::from([
            ("walk".to_string(), 2),
            ("drive".to_string(), 1),
        ]));
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a DRIVE-mode edge list
        let edge = Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "drive", "walk"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(!is_valid);
    }

    #[test]
    fn test_valid_allowed_modes() {
        // testing validitity of traversing a "walk" edge when the constraint allows only
        // "walk" and "transit" modes. this should be valid.
        let mode_constraint = MultimodalFrontierConstraint::AllowedModes(HashSet::from([
            "walk".to_string(),
            "transit".to_string(),
        ]));
        let max_trip_legs = 3;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a walk-mode edge list
        let edge = Edge::new(1, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "transit", "walk"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_invalid_allowed_modes() {
        // testing validitity of traversing a "drive" edge when the constraint allows only
        // "walk" and "transit" modes. this should be invalid.
        let mode_constraint = MultimodalFrontierConstraint::AllowedModes(HashSet::from([
            "walk".to_string(),
            "transit".to_string(),
        ]));
        let max_trip_legs = 3;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a DRIVE-mode edge list
        let edge = Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "transit", "walk"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(!is_valid);
    }

    #[test]
    fn test_valid_subsequence_empty_state() {
        // testing validitity of traversing a "walk" edge for an initial state where "walk"
        // is a matching subsequence. should be valid.
        let mut trie = SubSequenceTrie::new();
        trie.insert_sequence(vec![
            "walk".to_string(),
            "transit".to_string(),
            "walk".to_string(),
        ]);
        let mode_constraint = MultimodalFrontierConstraint::ExactSequences(trie);
        let max_trip_legs = 3;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a walk-mode edge list
        let edge = Edge::new(1, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_valid_subsequence() {
        // testing validitity of traversing a "walk" edge for a "walk"->"transit" state where "walk"
        // is a matching subsequence. should be valid.
        let mut trie = SubSequenceTrie::new();
        trie.insert_sequence(vec![
            "walk".to_string(),
            "transit".to_string(),
            "walk".to_string(),
        ]);
        let mode_constraint = MultimodalFrontierConstraint::ExactSequences(trie);
        let max_trip_legs = 3;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a walk-mode edge list
        let edge = Edge::new(1, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "transit"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(is_valid);
    }

    #[test]
    fn test_invalid_subsequence() {
        // testing validitity of traversing a "walk" edge for a "walk"->"transit" state where "walk"->"transit"->"walk"
        // is NOT a matching subsequence. should be invalid.
        let mut trie = SubSequenceTrie::new();
        trie.insert_sequence(vec!["walk".to_string(), "transit".to_string()]);
        let mode_constraint = MultimodalFrontierConstraint::ExactSequences(trie);
        let max_trip_legs = 3;
        let (mam, mfm, state_model, mut state) = test_setup(
            vec![mode_constraint],
            "walk",
            &["walk", "bike", "drive", "tnc", "transit"],
            &["drive", "walk", "bike", "transit"],
            max_trip_legs,
        );

        // edge list one is a walk-mode edge list
        let edge = Edge::new(1, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0));

        inject_trip_legs(
            &["walk", "transit"],
            &mut state,
            &state_model,
            &mam.mode_to_state,
            max_trip_legs,
        );

        // test
        let is_valid = mfm
            .valid_frontier(&edge, None, &state, &state_model)
            .expect("test failed");
        assert!(!is_valid);
    }

    /// helper function to set up MultimodalFrontierModel test case assets
    fn test_setup(
        constraints: Vec<MultimodalFrontierConstraint>,
        this_mode: &str,
        modes: &[&str],
        edge_list_modes: &[&str],
        max_trip_legs: u64,
    ) -> (
        MultimodalTraversalModel,
        MultimodalFrontierModel,
        StateModel,
        Vec<StateVariable>,
    ) {
        let mtm = MultimodalTraversalModel::new_local(this_mode, max_trip_legs, modes, &[], true)
            .expect("test invariant failed");
        let state_model = StateModel::new(mtm.output_features());
        let mfm =
            MultimodalFrontierModel::new_local(max_trip_legs, modes, edge_list_modes, constraints)
                .expect("test invariant failed");
        let state = state_model
            .initial_state(None)
            .expect("test invariant failed");

        (mtm, mfm, state_model, state)
    }

    fn inject_trip_legs(
        legs: &[&str],
        state: &mut [StateVariable],
        state_model: &StateModel,
        mode_to_state: &MultimodalStateMapping,
        max_trip_legs: u64,
    ) {
        for (leg_idx, mode) in legs.iter().enumerate() {
            state_ops::set_leg_mode(state, leg_idx as u64, mode, &state_model, &mode_to_state)
                .expect("test invariant failed");
            state_ops::increment_active_leg_idx(state, &state_model, max_trip_legs)
                .expect("test invariant failed");
        }
    }
}
