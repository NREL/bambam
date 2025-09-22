use crate::model::{
    state::{
        fieldname, multimodal_state_ops as ops, variable, LegIdx, MultimodalMapping,
        MultimodalStateMapping,
    },
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

pub struct MultimodalAccessModel {
    pub mode: String,
    pub max_trip_legs: u64,
    pub mode_to_state: Arc<MultimodalStateMapping>,
    // pub route_id_mapping: Arc<MultimodalStateMapping>,
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
impl AccessModel for MultimodalAccessModel {
    fn state_features(&self) -> Vec<(String, StateVariableConfig)> {
        let active_leg = std::iter::once((
            fieldname::ACTIVE_LEG.to_string(),
            variable::active_leg_variable_config(),
        ));
        let leg_mode = (0..self.max_trip_legs).map(|idx| {
            let name = fieldname::leg_mode_fieldname(idx);
            let config = variable::leg_mode_variable_config();
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
        let leg_and_mode_opt = match ops::get_active_leg_idx(state, state_model)? {
            Some(leg_idx) => {
                let mode = ops::get_existing_leg_mode(
                    state,
                    leg_idx,
                    state_model,
                    self.max_trip_legs,
                    &self.mode_to_state,
                )?;
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
                let next_leg_idx =
                    ops::increment_active_leg_idx(state, state_model, self.max_trip_legs)?;
                ops::set_leg_mode(
                    state,
                    next_leg_idx,
                    &self.mode,
                    state_model,
                    &self.mode_to_state,
                )?;
            }
        };

        Ok(())
    }
}

impl MultimodalAccessModel {
    pub fn new(
        mode: String,
        max_trip_legs: u64,
        mode_to_state: Arc<MultimodalStateMapping>,
    ) -> MultimodalAccessModel {
        Self {
            mode,
            max_trip_legs,
            mode_to_state,
        }
    }

    /// builds a new [`MultimodalAccessModel`] from its data dependencies only.
    /// used in synchronous contexts like scripting or testing.
    pub fn new_local(
        mode: &str,
        max_trip_legs: u64,
        modes: &[&str],
        route_ids: &[&str],
    ) -> Result<MultimodalAccessModel, StateModelError> {
        let mode_to_state =
            MultimodalMapping::new(&modes.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                .map_err(|e| {
                StateModelError::BuildError(format!(
                    "while building MultimodalAccessModel, failure constructing mode mapping: {e}"
                ))
            })?;

        let mmm = MultimodalAccessModel::new(
            mode.to_string(),
            max_trip_legs,
            Arc::new(mode_to_state),
            // Arc::new(route_id_mapping),
        );
        Ok(mmm)
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
            let mode_key = fieldname::leg_mode_fieldname(idx);
            let route_key = fieldname::leg_route_id_fieldname(idx);
            apply_mapping_for_serialization(state_json, &mode_key, idx, &self.mode_to_state)?;
            // apply_mapping_for_serialization(state_json, &route_key, idx, &self.route_id_mapping)?;
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
    mapping: &MultimodalStateMapping,
) -> Result<(), StateModelError> {
    if let Some(v) = state_json.get_mut(name) {
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
    use super::MultimodalAccessModel;
    use crate::model::state::{fieldname, multimodal_state_ops as ops, LegIdx, MultimodalMapping};
    use routee_compass_core::model::{
        access::AccessModel,
        network::{Edge, Vertex},
        state::{StateModel, StateVariable},
    };
    use std::sync::Arc;
    use uom::si::f64::Length;

    // an initialized trip that has not begun should have active leg of None and
    // leg_0_mode of None.
    #[test]
    fn test_initialize_trip_access() {
        let test_mode = "walk";
        let mmm = MultimodalAccessModel::new_local("walk", 1, &["walk"], &[])
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
        let mmm = MultimodalAccessModel::new_local("walk", 1, &["walk"], &[])
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
        let mmm_walk = MultimodalAccessModel::new_local("walk", 2, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let mmm_bike = MultimodalAccessModel::new_local("bike", 2, &["bike", "walk"], &[])
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
        let mmm_walk = MultimodalAccessModel::new_local("walk", 1, &["bike", "walk"], &[])
            .expect("test invariant failed, model constructor had error");
        let mmm_bike = MultimodalAccessModel::new_local("bike", 1, &["bike", "walk"], &[])
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
        mmm: &MultimodalAccessModel,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<(), String> {
        let active_leg = ops::get_active_leg_idx(&state, &state_model)
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
        mmm: &MultimodalAccessModel,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<(), String> {
        let active_leg_opt = ops::get_active_leg_idx(&state, &state_model)
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
                let active_mode = ops::get_existing_leg_mode(&state, leg_idx, &state_model, mmm.max_trip_legs, &mmm.mode_to_state)
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
