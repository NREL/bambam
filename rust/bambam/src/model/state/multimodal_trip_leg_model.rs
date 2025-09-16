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
            state_variable::active_leg(),
        ));
        let leg_mode = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_mode_fieldname(idx);
            let config = super::state_variable::leg_mode();
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
        ]
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        let active_leg = std::iter::once((
            fieldname::ACTIVE_LEG.to_string(),
            state_variable::active_leg(),
        ));
        let leg_mode = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_mode_fieldname(idx);
            let config = super::state_variable::leg_mode();
            (name, config)
        });
        let leg_dist = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_distance_fieldname(idx);
            let config = super::state_variable::multimodal_distance(None);
            (name, config)
        });
        let leg_time = (0..self.max_trip_legs).map(|idx| {
            let name = super::fieldname::leg_time_fieldname(idx);
            let config = super::state_variable::multimodal_time(None);
            (name, config)
        });
        let mode_dist = std::iter::once((
            super::fieldname::mode_distance_fieldname(&self.mode),
            super::state_variable::multimodal_distance(None),
        ));
        let mode_time = std::iter::once((
            super::fieldname::mode_time_fieldname(&self.mode),
            super::state_variable::multimodal_time(None),
        ));
        active_leg
            .chain(leg_mode)
            .chain(leg_dist)
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
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::model::state::{
        fieldname, multimodal_trip_leg_model::MultimodalTripLegModel, MultimodalMapping,
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
        let mode_mapping = Arc::new(
            MultimodalMapping::new(&[String::from("bike"), String::from("walk")])
                .expect("test invariant failed: unable to construct mapping"),
        );
        let route_id_mapping = Arc::new(
            MultimodalMapping::new(&[String::from("1")])
                .expect("test invariant failed: unable to construct mapping"),
        );
        let this_mode = "walk".to_string();
        let mmm = MultimodalTripLegModel::new(
            this_mode.clone(),
            1,
            mode_mapping.clone(),
            route_id_mapping,
        );
        let state_model = StateModel::new(mmm.state_features());
        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        let result_idx = mmm
            .get_active_leg_idx(&state, &state_model)
            .expect("failure getting active leg index");

        // we have no accessor for the mode, but can confirm, the state should have
        // two variables, both set to the EMPTY value of -1.0.
        assert_eq!(state, vec![StateVariable(-1.0), StateVariable(-1.0)]);
    }

    // in a scenario with walk and bike mode, using an AccessModel for walk mode,
    // if we start a trip, we should assign 'walk' to the first leg and the active
    // leg should be 0.
    #[test]
    fn test_start_trip_access() {
        let mode_mapping = Arc::new(
            MultimodalMapping::new(&[String::from("bike"), String::from("walk")])
                .expect("test invariant failed: unable to construct mapping"),
        );
        let route_id_mapping = Arc::new(
            MultimodalMapping::new(&[String::from("1")])
                .expect("test invariant failed: unable to construct mapping"),
        );
        let test_mode = "walk".to_string();
        let mmm = MultimodalTripLegModel::new(
            test_mode.clone(),
            1,
            mode_mapping.clone(),
            route_id_mapping,
        );
        let state_model = StateModel::new(mmm.state_features());

        let trajectory = (
            &Vertex::new(0, 0.0, 0.0),
            &Edge::new(0, 0, 0, 1, Length::new::<uom::si::length::meter>(1000.0)),
            &Vertex::new(1, 0.01, 0.0),
            &Edge::new(0, 1, 1, 2, Length::new::<uom::si::length::meter>(1000.0)),
            &Vertex::new(2, 0.02, 0.0),
        );
        let mut state = state_model
            .initial_state()
            .expect("test invariant failed: unable to create state");

        mmm.access_edge(trajectory, &mut state, &state_model)
            .expect("access failed");

        let json = &state_model
            .serialize_state(&state, false)
            .expect("unable to serialize state");

        let active_leg = mmm
            .get_active_leg_idx(&state, &state_model)
            .expect("failure getting active leg index")
            .expect("active leg is not set");

        let leg_0_mode = mmm
            .get_leg_mode(&state, active_leg, &state_model)
            .expect(&format!("failure getting mode for leg {active_leg}"));

        assert_eq!(active_leg, 0);
        assert_eq!(leg_0_mode, &test_mode);
    }

    #[test]
    fn test_switch_trip_mode_access() {
        todo!()
    }

    #[test]
    fn test_initialize_trip_traversal() {
        todo!()
    }

    #[test]
    fn test_start_trip_traversal() {
        todo!()
    }
}
