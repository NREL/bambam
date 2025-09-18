use crate::model::{
    state::{
        fieldname, multimodal_state_ops, multimodal_state_ops as ops, variable, LegIdx,
        MultimodalMapping,
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

/// maps edge_time values to the correct mode and leg accumulators during traversal.
///
/// while the broader design of bambam assumes one travel mode per edge list, this model
/// instead assumes it can use some shared notion of a mapping from mode name to a numeric label
/// across edge lists.
pub struct MultimodalTraversalModel {
    pub mode: String,
    pub max_trip_legs: u64,
    pub mode_mapping: Arc<MultimodalMapping<String, i64>>,
}

/// Applies the multimodal leg + mode-specific accumulator updates during
/// edge_traversal.
impl TraversalModel for MultimodalTraversalModel {
    fn name(&self) -> String {
        format!("Multimodal Traversal Model ({})", self.mode)
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
            // variable::active_leg_input_feature(),
        ]
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        let leg_dist = (0..self.max_trip_legs).map(|idx| {
            let name = fieldname::leg_distance_fieldname(idx);
            let config = variable::multimodal_distance_variable_config(None);
            (name, config)
        });
        let leg_time = (0..self.max_trip_legs).map(|idx| {
            let name = fieldname::leg_time_fieldname(idx);
            let config = variable::multimodal_time_variable_config(None);
            (name, config)
        });

        let mode_dist = self.mode_mapping.get_categories().iter().map(|mode| {
            let name = fieldname::mode_distance_fieldname(mode);
            let config = variable::multimodal_distance_variable_config(None);
            (name, config)
        });

        let mode_time = self.mode_mapping.get_categories().iter().map(|mode| {
            let name = fieldname::mode_time_fieldname(mode);
            let config = variable::multimodal_time_variable_config(None);
            (name, config)
        });

        // let mode_dist = std::iter::once((
        //     fieldname::mode_distance_fieldname(&self.mode),
        //     variable::multimodal_distance_variable_config(None),
        // ));
        // let mode_time = std::iter::once((
        //     fieldname::mode_time_fieldname(&self.mode),
        //     variable::multimodal_time_variable_config(None),
        // ));
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
        let leg_idx = ops::get_active_leg_idx(state, state_model)?.ok_or_else(|| {
            multimodal_state_ops::error_inactive_state_traversal(state, state_model)
        })?;
        let distance: Length = state_model.get_distance(state, fieldname::EDGE_DISTANCE)?;
        let time: Time = state_model.get_time(state, fieldname::EDGE_TIME)?;
        let mode = ops::get_leg_mode(
            state,
            leg_idx,
            state_model,
            self.max_trip_legs,
            &self.mode_mapping,
        )?;
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

impl MultimodalTraversalModel {
    /// builds a new traversal model, associated with a specific edge list and travel mode.
    /// compatible with mode mappings shared from the upstream traversal model service or
    /// built just for this case.
    pub fn new(
        mode: String,
        max_trip_legs: u64,
        mode_mapping: Arc<MultimodalMapping<String, i64>>,
    ) -> MultimodalTraversalModel {
        Self {
            mode,
            max_trip_legs,
            mode_mapping,
        }
    }

    /// builds a new [`MultimodalTripLegModel`] from its data dependencies only.
    /// used in synchronous contexts like scripting or testing.
    pub fn new_local(
        mode: &str,
        max_trip_legs: u64,
        modes: &[&str],
    ) -> Result<MultimodalTraversalModel, StateModelError> {
        let mode_mapping =
            MultimodalMapping::new(&modes.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                .map_err(|e| {
                StateModelError::BuildError(format!(
                    "while building MultimodalTripLegModel, failure constructing mode mapping: {e}"
                ))
            })?;

        let mmm =
            MultimodalTraversalModel::new(mode.to_string(), max_trip_legs, Arc::new(mode_mapping));
        Ok(mmm)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::MultimodalTraversalModel;
    use crate::model::state::{multimodal_state_ops as ops, LegIdx, MultimodalMapping};
    use routee_compass_core::{
        model::{
            network::{Edge, Vertex},
            state::{StateModel, StateVariable},
            traversal::TraversalModel,
        },
        testing::mock::traversal_model::TestTraversalModel,
    };
    use uom::si::f64::Length;

    #[test]
    fn test_initialize_trip_traversal() {
        let available_modes = ["walk", "bike", "drive"];
        let max_trip_legs = 4;
        let tm = Arc::new(
            MultimodalTraversalModel::new_local("walk", max_trip_legs, &available_modes)
                .expect("test invariant failed, model constructor had error"),
        );
        let test_tm = TestTraversalModel::new(tm.clone())
            .expect("test invariant failed, unable to produce a test model");

        let state_model = StateModel::new(test_tm.output_features());

        let state = state_model
            .initial_state()
            .expect("test invariant failed: state model could not create initial state");

        // as a head check, we can also inspect the serialized access state JSON in the logs
        let state_json = state_model
            .serialize_state(&state, false)
            .expect("state serialization failed");
        println!(
            "{}",
            serde_json::to_string_pretty(&state_json).unwrap_or_default()
        );

        // ASSERTION 1: state has the expected length given the provided number of trip legs + modes
        let expected_len = {
            let input_features = 2; // edge_time, trip_time
            let leg_fields = 2;
            let mode_fields = 2;
            input_features
                + available_modes.len() * mode_fields
                + max_trip_legs as usize * leg_fields
        };
        assert_eq!(state.len(), expected_len);

        // ASSERTION 2: confirm each leg's dist/time keys exist and values were set with zeroes
        for leg_idx in (0..max_trip_legs) {
            let dist = ops::get_leg_distance(&state, leg_idx, &state_model)
                .expect(&format!("unable to get leg attribute for leg {leg_idx}"));
            let time = ops::get_leg_time(&state, leg_idx, &state_model)
                .expect(&format!("unable to get leg attribute for leg {leg_idx}"));
            assert_eq!(dist.value, 0.0);
            assert_eq!(time.value, 0.0);
        }
    }

    #[test]
    fn test_start_trip_traversal() {
        let model = MultimodalTraversalModel::new_local("walk", 1, &["walk"])
            .expect("test invariant failed, model constructor had error");

        // (0) -[0]-> (1)
        let trajectory = mock_trajectory(0, 0, 0);

        // let fields = [
        //     "walk_distance",
        //     "walk_time",
        //     "leg_0_mode",
        //     "leg_0_distance",
        //     "leg_0_time",
        //     "leg_0_route_id",
        // ];
        // assert_active_leg(Some(0), &state, &state_model);
        // assert_active_mode(
        //     Some("walk"),
        //     &state,
        //     &state_model,
        //     tm.max_trip_legs,
        //     &tm.mode_mapping,
        // );
    }

    /// helper to create trajectories spaced apart evenly along a line with segments of uniform length
    fn mock_trajectory(
        start_vertex: usize,
        start_edge: usize,
        e1_edgelist: usize,
    ) -> (Vertex, Edge, Vertex) {
        let v1 = start_vertex;
        let v2 = v1 + 1;
        let x1 = (v1 as f32) * 0.01;
        let x2 = (v2 as f32) * 0.01;

        let e1 = start_edge;
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
        )
    }

    fn assert_active_leg(
        leg_idx: Option<LegIdx>,
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
        state: &[StateVariable],
        state_model: &StateModel,
        max_trip_legs: u64,
        mode_mapping: &MultimodalMapping<String, i64>,
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
                let active_mode = ops::get_leg_mode(&state, leg_idx, &state_model, max_trip_legs, &mode_mapping)
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
