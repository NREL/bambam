use super::fieldname;
use crate::model::{
    state::{multimodal_state_ops, state_variable, LegIdx, MultimodalMapping},
    transit_old::gtfs_old::route,
};
use itertools::Itertools;
use routee_compass_core::model::{
    access::{AccessModel, AccessModelError},
    label::Label,
    network::{Edge, Vertex, VertexId},
    state::{InputFeature, StateModel, StateModelError, StateVariable, StateVariableConfig},
    traversal::{TraversalModel, TraversalModelError},
};
use serde_json::json;
use std::sync::Arc;
use uom::si::f64::{Length, Time};

pub struct MultimodalTripLegModel {
    pub mode: String,
    pub max_trip_legs: u64,
    pub mode_mapping: Arc<MultimodalMapping<String, i64>>,
    pub route_id_mapping: Arc<MultimodalMapping<String, i64>>,
}

/// Handles any mode transition occurring by accessing a new edge.
///
/// MultimodalFrontierModel should guard against exceeding the maximum allowed
/// number of trip legs. here, attempting to do so results in an error.
///
/// This model does not modify any metric accumulators such as distance or time.
///
/// It should appear _first_ in a stack of AccessModels so that the active leg and
/// mode are always updated by downstream AccessModels + TraversalModels.
impl AccessModel for MultimodalTripLegModel {
    fn state_features(&self) -> Vec<(String, StateVariableConfig)> {
        let active_leg = std::iter::once((
            fieldname::ACTIVE_LEG.to_string(),
            state_variable::active_leg_variable_config(),
        ));
        let leg_mode = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_mode_fieldname(idx);
            let config = super::state_variable::leg_mode_variable_config();
            (name, config)
        });
        active_leg.chain(leg_mode).collect_vec()
    }

    fn access_edge(
        &self,
        traversal: (&Vertex, &Edge, &Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), AccessModelError> {
        // grab the leg_idx and leg mode if it exists
        let leg_and_mode_opt = match self.get_active_leg_idx(state, state_model)? {
            Some(leg_idx) => {
                let mode = self.get_leg_mode(state, leg_idx, state_model)?;
                Some((leg_idx, mode))
            }
            None => None,
        };

        match leg_and_mode_opt {
            Some((_, mode)) if mode == self.mode => {
                // leg exists but no change in mode -> return early
            }
            _ => {
                // no leg assigned or a change in mode -> add the new leg
                let next_leg_idx = self.increment_active_leg_idx(state, state_model)?;
                self.set_leg_mode(state, next_leg_idx, &self.mode, state_model)?;
            }
        };

        Ok(())
    }
}

/// Applies the multimodal leg + mode-specific accumulator updates during
/// edge_traversal.
///
/// Should be _last_ in a stack of TraversalModels so that all edge metrics
/// are current before copying to the leg + mode accumulators.
impl TraversalModel for MultimodalTripLegModel {
    fn name(&self) -> String {
        format!("Multimodal Trip Leg Traversal Model ({})", self.mode)
    }

    fn input_features(&self) -> Vec<InputFeature> {
        vec![
            InputFeature::Distance {
                name: fieldname::EDGE_DISTANCE.to_string(),
                unit: None,
            },
            InputFeature::Time {
                name: fieldname::EDGE_TIME.to_string(),
                unit: None,
            },
            state_variable::active_leg_input_feature(),
        ]
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        // let active_leg = std::iter::once((
        //     fieldname::ACTIVE_LEG.to_string(),
        //     state_variable::active_leg(),
        // ));
        // let leg_mode = (0..self.max_trip_legs).map(|idx| {
        //     let name = super::fieldname::leg_mode_fieldname(idx);
        //     let config = super::state_variable::leg_mode();
        //     (name, config)
        // });
        let leg_dist = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_distance_fieldname(idx);
            let config = super::state_variable::multimodal_distance_variable_config(None);
            (name, config)
        });
        let leg_time = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_time_fieldname(idx);
            let config = super::state_variable::multimodal_time_variable_config(None);
            (name, config)
        });
        let mode_dist = std::iter::once((
            super::fieldname::mode_distance_fieldname(&self.mode),
            super::state_variable::multimodal_distance_variable_config(None),
        ));
        let mode_time = std::iter::once((
            super::fieldname::mode_time_fieldname(&self.mode),
            super::state_variable::multimodal_time_variable_config(None),
        ));
        // active_leg
        //     .chain(leg_mode)
        leg_dist
            .chain(leg_time)
            .chain(mode_dist)
            .chain(mode_time)
            .collect_vec()
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let (_, edge, _) = trajectory;
        let leg_idx = self
            .get_active_leg_idx(state, state_model)?
            .ok_or_else(|| {
                multimodal_state_ops::error_inactive_state_traversal(state, state_model)
            })?;
        let distance: Length = state_model.get_distance(state, fieldname::EDGE_DISTANCE)?;
        let time: Time = state_model.get_time(state, fieldname::EDGE_TIME)?;
        let mode = self.get_leg_mode(state, leg_idx, state_model)?;
        let d_leg = fieldname::leg_distance_fieldname(leg_idx);
        let d_mode = fieldname::mode_distance_fieldname(mode);
        let t_leg = fieldname::leg_distance_fieldname(leg_idx);
        let t_mode = fieldname::mode_time_fieldname(mode);
        state_model.add_distance(state, &d_leg, &distance)?;
        state_model.add_distance(state, &d_mode, &distance)?;
        state_model.add_time(state, &t_leg, &time)?;
        state_model.add_time(state, &t_mode, &time)?;
        Ok(())
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        // does not support A*-style estimation
        Ok(())
    }
    //
}

impl MultimodalTripLegModel {
    pub fn new(
        mode: String,
        max_trip_legs: u64,
        mode_mapping: Arc<MultimodalMapping<String, i64>>,
        route_id_mapping: Arc<MultimodalMapping<String, i64>>,
    ) -> MultimodalTripLegModel {
        Self {
            mode,
            max_trip_legs,
            mode_mapping,
            route_id_mapping,
        }
    }

    /// builds a new [`MultimodalTripLegModel`] from its data dependencies only.
    /// used in synchronous contexts like scripting or testing.
    pub fn new_local(
        mode: &str,
        max_trip_legs: u64,
        modes: &[&str],
        route_ids: &[&str],
    ) -> Result<MultimodalTripLegModel, StateModelError> {
        let mode_mapping =
            MultimodalMapping::new(&modes.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                .map_err(|e| {
                StateModelError::BuildError(format!(
                    "while building MultimodalTripLegModel, failure constructing mode mapping: {e}"
                ))
            })?;
        let route_id_mapping = MultimodalMapping::new(
            &route_ids
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .map_err(|e| {
            StateModelError::BuildError(format!(
                "while building MultimodalTripLegModel, failure constructing route_id mapping: {e}"
            ))
        })?;

        let mmm = MultimodalTripLegModel::new(
            mode.to_string(),
            max_trip_legs,
            Arc::new(mode_mapping),
            Arc::new(route_id_mapping),
        );
        Ok(mmm)
    }

    /// inspect the current active leg for a trip
    pub fn get_active_leg_idx(
        &self,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<Option<LegIdx>, StateModelError> {
        let leg_i64 = state_model.get_custom_i64(state, fieldname::ACTIVE_LEG)?;
        if leg_i64 < 0 {
            Ok(None)
        } else {
            let leg_u64 = leg_i64.try_into()
                .map_err(|e| StateModelError::RuntimeError(format!("internal error: while getting active trip leg, unable to parse {leg_i64} as a u64")))?;
            Ok(Some(leg_u64))
        }
    }

    /// report if any trip data has been recorded for the given trip leg.
    /// this uses the fact that any trip leg must have a leg mode, and leg modes
    /// are stored with non-negative integer values, negative denotes "empty".
    /// see [`super::state_variable`] for the leg mode variable configuration.
    pub fn contains_leg(
        &self,
        state: &mut [StateVariable],
        leg_idx: LegIdx,
        state_model: &StateModel,
    ) -> Result<bool, StateModelError> {
        self.validate_leg_idx(leg_idx)?;
        let name = fieldname::leg_mode_fieldname(leg_idx);
        let label = state_model.get_custom_i64(state, &name)?;
        Ok(label >= 0)
    }

    /// get the travel mode for a leg.
    pub fn get_leg_mode(
        &self,
        state: &[StateVariable],
        leg_idx: LegIdx,
        state_model: &StateModel,
    ) -> Result<&str, StateModelError> {
        self.validate_leg_idx(leg_idx)?;
        let name = fieldname::leg_mode_fieldname(leg_idx);
        let label = state_model.get_custom_i64(state, &name)?;
        if label < 0 {
            Err(StateModelError::RuntimeError(format!(
                "Internal Error: get_leg_mode called on leg idx {} but mode label is not set (stored as {})",
                leg_idx,
                label
            )))
        } else {
            self.mode_mapping
                .get_categorical(label)?
                .ok_or_else(|| {
                    StateModelError::RuntimeError(format!(
                        "internal error, leg {} has invalid mode label {}",
                        leg_idx, label
                    ))
                })
                .map(|s| s.as_str())
        }
    }

    pub fn get_leg_distance(
        &self,
        state: &mut [StateVariable],
        leg_idx: LegIdx,
        state_model: &StateModel,
    ) -> Result<Length, StateModelError> {
        let name = fieldname::leg_distance_fieldname(leg_idx);
        state_model.get_distance(state, &name)
    }

    pub fn get_leg_time(
        &self,
        state: &[StateVariable],
        leg_idx: LegIdx,
        state_model: &StateModel,
    ) -> Result<Time, StateModelError> {
        let name = fieldname::leg_time_fieldname(leg_idx);
        state_model.get_time(state, &name)
    }

    pub fn get_leg_route_id(
        &self,
        state: &[StateVariable],
        leg_idx: LegIdx,
        state_model: &StateModel,
    ) -> Result<Option<&String>, StateModelError> {
        let name = fieldname::leg_route_id_fieldname(leg_idx);
        let route_id_label = state_model.get_custom_i64(state, &name)?;
        let route_id = self.route_id_mapping.get_categorical(route_id_label)?;
        Ok(route_id)
    }

    /// validates leg_idx values, which must be in range [0, max_trip_legs)
    fn validate_leg_idx(&self, leg_idx: LegIdx) -> Result<(), StateModelError> {
        if leg_idx >= self.max_trip_legs {
            Err(StateModelError::RuntimeError(format!(
                "invalid leg id {leg_idx} >= max leg id {}",
                self.max_trip_legs
            )))
        } else {
            Ok(())
        }
    }

    /// increments the value at [`fieldname::ACTIVE_LEG`].
    /// when ACTIVE_LEG is negative (no active leg), it becomes zero.
    /// when it is a number in [0, max_legs-1), it is incremented by one.
    /// returns the new index value.
    pub fn increment_active_leg_idx(
        &self,
        state: &mut [StateVariable],
        state_model: &StateModel,
    ) -> Result<LegIdx, StateModelError> {
        // get the index of the next leg
        let next_leg_idx_u64 = match self.get_active_leg_idx(state, state_model)? {
            Some(leg_idx) => {
                let next = leg_idx + 1;
                self.validate_leg_idx(next)?;
                next
            }
            None => 0,
        };
        // as an i64, to match the storage format
        let next_leg_idx: i64 = next_leg_idx_u64.try_into().map_err(|e| {
            StateModelError::RuntimeError(format!(
                "internal error: while getting active trip leg, unable to parse {next_leg_idx_u64} as a i64"
            ))
        })?;

        // increment the value in the state vector
        state_model.set_custom_i64(state, fieldname::ACTIVE_LEG, &next_leg_idx)?;
        Ok(next_leg_idx_u64)
    }

    /// sets the mode value for the given leg. performs mapping from Mode -> i64 which is
    /// the storage type for Mode in the state vector.
    pub fn set_leg_mode(
        &self,
        state: &mut [StateVariable],
        leg_idx: LegIdx,
        mode: &str,
        state_model: &StateModel,
    ) -> Result<(), StateModelError> {
        let mode_label = self.mode_mapping.get_label(mode).ok_or_else(|| {
            StateModelError::RuntimeError(format!("mode mapping has no entry for '{}' mode", mode))
        })?;
        let name = fieldname::leg_mode_fieldname(leg_idx);
        state_model.set_custom_i64(state, &name, mode_label)
    }

    /// modifies a state serialization so that values related to multimodal access modeling
    /// have been re-mapped to their categorical values
    pub fn serialize_mapping_values(
        &self,
        state_json: &mut serde_json::Value,
        state: &[StateVariable],
        state_model: &StateModel,
        accumulators_only: bool,
    ) -> Result<(), StateModelError> {
        // use mappings to map any multimodal state values to their respective categoricals
        for idx in (0..self.max_trip_legs) {
            // re-map leg mode
            let mode_key = super::fieldname::leg_mode_fieldname(idx);
            let route_key = super::fieldname::leg_route_id_fieldname(idx);
            apply_mapping_for_serialization(state_json, &mode_key, idx, &self.mode_mapping)?;
            apply_mapping_for_serialization(state_json, &route_key, idx, &self.route_id_mapping)?;
        }

        Ok(())
    }
}

/// helper function for applying the label/categorical mapping in the
/// context of serializing a value on an output multimodal search state JSON.
fn apply_mapping_for_serialization(
    state_json: &mut serde_json::Value,
    name: &str,
    leg_idx: LegIdx,
    mapping: &MultimodalMapping<String, i64>,
) -> Result<(), StateModelError> {
    if let Some(v) = state_json.get_mut(&name) {
        let label = v.as_i64().ok_or_else(|| {
            StateModelError::RuntimeError(format!(
                "unable to get label (i64) value for leg index, key {leg_idx}, {name}"
            ))
        })?;
        if label < 0 {
            *v = json![""]; // no mode assigned
        } else {
            let cat = mapping.get_categorical(label)?.ok_or_else(|| {
                StateModelError::RuntimeError(format!(
                    "while serializing multimodal state, mapping failed for name, leg index, label: {name}, {leg_idx}, {label}"
                ))
            })?;
            *v = json![cat.to_string()];
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::model::state::{
        fieldname, multimodal_trip_leg_model::MultimodalTripLegModel, LegIdx, MultimodalMapping,
    };
    use routee_compass_core::model::{
        access::AccessModel,
        network::{Edge, Vertex},
        state::{StateModel, StateVariable},
    };
    use uom::si::f64::Length;

    // an initialized trip that has not begun should have active leg of None and
    // leg_0_mode of None.
    #[test]
    fn test_initialize_trip_access() {
        let test_mode = "walk";
        let mmm = MultimodalTripLegModel::new_local("walk", 1, &["walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let state_model = StateModel::new(mmm.state_features());

        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        // ASSERTION 1: there should be no active leg index, no trip has started.
        assert_active_leg(None, &mmm, &state, &state_model).expect("assertion 1 failed");

        // ASSERTION 2: as we have no active leg index, the state vector should be in it's
        // initial state. this should be a Vec of size 2 with both values set to 'EMPTY' (-1.0).
        let expected = state_model
            .initial_state()
            .expect("test invariant failed: cannot build initial state");
        assert_eq!(state, expected);
        assert_eq!(state, vec![StateVariable(-1.0), StateVariable(-1.0)]);
    }

    // in a scenario with walk and bike mode, using an AccessModel for walk mode,
    // if we start a trip, we should assign 'walk' to the first leg and the active
    // leg should be 0.
    #[test]
    fn test_start_trip_access() {
        let test_mode = "walk";
        let mmm = MultimodalTripLegModel::new_local("walk", 1, &["walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let state_model = StateModel::new(mmm.state_features());

        let t1 = mock_trajectory(0, 0, 0, 0);
        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        mmm.access_edge(
            (&t1.0, &t1.1, &t1.2, &t1.3, &t1.4),
            &mut state,
            &state_model,
        )
        .expect("access failed");

        // ASSERTION 1: by accessing a traversal, we must have transitioned from our initial state
        // to a state with exactly one trip leg.
        assert_active_leg(Some(0), &mmm, &state, &state_model).expect("assertion 1 failed");

        // ASSERTION 2: the trip leg should be associated with the mode that the AccessModel sets.
        assert_active_mode(Some(test_mode), &mmm, &state, &state_model)
            .expect("assertion 2 failed");
    }

    #[test]
    fn test_switch_trip_mode_access() {
        // create an access model for two edge lists, "walk" and "bike" topology
        let mmm_walk = MultimodalTripLegModel::new_local("walk", 2, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let mmm_bike = MultimodalTripLegModel::new_local("bike", 2, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");

        // build state model and initial search state
        assert_eq!(
            mmm_walk.state_features(),
            mmm_bike.state_features(),
            "test invariant failed: models should have matching state features"
        );
        let state_model = StateModel::new(mmm_walk.state_features());
        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        // access edge 2 in walk mode, access edge 3 in bike mode
        // (0) -[0]-> (1) -[1]-> (2) -[2]-> (3) where
        //   - edge list 0 has edges 0 and 1, uses walk-mode access model
        //   - edge list 1 has edge 2, uses bike-mode access model
        let t1 = mock_trajectory(0, 0, 0, 0);
        let t2 = mock_trajectory(1, 1, 0, 1);

        // ASSERTION 1: trip enters "walk" mode after accessing edge 1 on edge list 0
        mmm_walk
            .access_edge(
                (&t1.0, &t1.1, &t1.2, &t1.3, &t1.4),
                &mut state,
                &state_model,
            )
            .expect("access failed");
        assert_active_leg(Some(0), &mmm_walk, &state, &state_model).expect("assertion 1 failed");
        assert_active_mode(Some("walk"), &mmm_walk, &state, &state_model)
            .expect("assertion 1 failed");

        // ASSERTION 2: trip enters "bike" mode after accessing edge 2 on edge list 1
        mmm_bike
            .access_edge(
                (&t2.0, &t2.1, &t2.2, &t2.3, &t2.4),
                &mut state,
                &state_model,
            )
            .expect("access failed");
        assert_active_leg(Some(1), &mmm_walk, &state, &state_model).expect("assertion 2 failed");
        assert_active_mode(Some("bike"), &mmm_walk, &state, &state_model)
            .expect("assertion 2 failed");

        // as a head check, we can also inspect the serialized access state JSON in the logs
        let mut state_json = state_model
            .serialize_state(&state, false)
            .expect("state serialization failed");
        mmm_walk
            .serialize_mapping_values(&mut state_json, &state, &state_model, false)
            .expect("state serialization failed");
        println!(
            "{}",
            serde_json::to_string_pretty(&state_json).unwrap_or_default()
        );
    }

    #[test]
    fn test_switch_exceeds_max_legs() {
        // create an access model for two edge lists, "walk" and "bike" topology
        // but, here, we limit trip legs to 1, so our trip should not be able to transition to bike
        let mmm_walk = MultimodalTripLegModel::new_local("walk", 1, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let mmm_bike = MultimodalTripLegModel::new_local("bike", 1, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");

        // build state model and initial search state
        assert_eq!(
            mmm_walk.state_features(),
            mmm_bike.state_features(),
            "test invariant failed: models should have matching state features"
        );
        let state_model = StateModel::new(mmm_walk.state_features());
        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        // the two trajectories concatenate together into the sequence
        // (0) -[0]-> (1) -[1]-> (2) -[2]-> (3)
        // where
        //   - edge list 0 has edges 0 and 1, uses walk-mode access model
        //   - edge list 1 has edge 2, uses bike-mode access model
        let t1 = mock_trajectory(0, 0, 0, 0);
        let t2 = mock_trajectory(1, 1, 0, 1);

        // establish the trip state on "walk"-mode travel
        mmm_walk
            .access_edge(
                (&t1.0, &t1.1, &t1.2, &t1.3, &t1.4),
                &mut state,
                &state_model,
            )
            .expect("access failed");

        // ASSERTION 1: trip tries to enter "bike" mode after accessing edge 2 on edge list 1,
        // but this should result in an error, as we have restricted the max number of trip legs to 1.
        let result = mmm_bike.access_edge(
            (&t2.0, &t2.1, &t2.2, &t2.3, &t2.4),
            &mut state,
            &state_model,
        );
        match result {
            Ok(()) => panic!("assertion 1 failed"),
            Err(e) => assert!(format!("{e}").contains("invalid leg id 1 >= max leg id 1")),
        }
    }

    #[test]
    fn test_initialize_trip_traversal() {
        todo!()
    }

    #[test]
    fn test_start_trip_traversal() {
        todo!()
    }

    /// helper to create trajectories spaced apart evenly along a line with segments of uniform length
    fn mock_trajectory(
        start_vertex: usize,
        start_edge: usize,
        e1_edgelist: usize,
        e2_edgelist: usize,
    ) -> (Vertex, Edge, Vertex, Edge, Vertex) {
        let v1 = start_vertex;
        let v2 = v1 + 1;
        let v3 = v2 + 1;
        let x1 = (v1 as f32) * 0.01;
        let x2 = (v2 as f32) * 0.01;
        let x3 = (v3 as f32) * 0.01;

        let e1 = start_edge;
        let e2 = e1 + 1;
        (
            Vertex::new(v1, x1, 0.0),
            Edge::new(
                e1_edgelist,
                e1,
                v1,
                v2,
                Length::new::<uom::si::length::meter>(1000.0),
            ),
            Vertex::new(v2, x2, 0.0),
            Edge::new(
                e2_edgelist,
                e2,
                v2,
                v3,
                Length::new::<uom::si::length::meter>(1000.0),
            ),
            Vertex::new(v3, x3, 0.0),
        )
    }

    fn assert_active_leg(
        leg_idx: Option<LegIdx>,
        mmm: &MultimodalTripLegModel,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<(), String> {
        let active_leg = mmm
            .get_active_leg_idx(&state, &state_model)
            .expect("failure getting active leg index");

        match (leg_idx, active_leg) {
            (None, None) => {
                // no active leg testing against no active mode, ok
                Ok(())
            }
            (None, Some(leg_idx)) => {
                Err(format!("assert_active_leg failure: we are expecting no active leg, but state has leg index of {leg_idx}"))
            }
            (Some(idx), None) => {
                Err(format!("assert_active_leg failure: we are expecting active leg index {idx}, but state has no active leg"))
            }
            (Some(test_idx), Some(active_leg_idx)) => {
                if test_idx != active_leg_idx {
                    Err(format!("expected active leg index of {active_leg_idx} to be {test_idx}"))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn assert_active_mode(
        mode: Option<&str>,
        mmm: &MultimodalTripLegModel,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<(), String> {
        let active_leg_opt = mmm
            .get_active_leg_idx(&state, &state_model)
            .expect("failure getting active leg index");

        match (mode, active_leg_opt) {
            (None, None) => {
                // no active leg testing against no active mode, ok
                Ok(())
            }
            (None, Some(leg_idx)) => {
                Err(format!("assert_active_mode failure: we are expecting no active mode, but state has leg index of {leg_idx}"))
            }
            (Some(m), None) => {
                Err(format!("assert_active_mode failure: we are expecting an active mode, but state has no active leg"))
            }
            (Some(test_mode), Some(leg_idx)) => {
                let active_mode = mmm
                    .get_leg_mode(&state, leg_idx, &state_model)
                    .expect(&format!("failure getting mode for leg {leg_idx}"));

                if active_mode != test_mode {
                    Err(format!("expected active leg mode of {active_mode} to be {test_mode}"))
                } else {
                    Ok(())
                }

            }
        }
    }
}
