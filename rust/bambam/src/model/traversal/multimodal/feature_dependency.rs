use crate::model::{
    bambam_feature,
    traversal::multimodal::{DependencyUnitType, FeatureDependencyConfig},
};
use itertools::Itertools;
use routee_compass_core::model::{
    state::{
        CustomVariableConfig, InputFeature, StateModel, StateModelError, StateVariable,
        StateVariableConfig,
    },
    traversal::TraversalModelError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uom::si::f64::Time;

#[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(untagged, rename_all = "snake_case")]
pub struct FeatureDependency {
    /// state variable that is read from to produce destination features
    pub input_feature: InputFeature,
    /// the list of fields to write to with the given state variable configuration
    pub destination_features: Vec<(String, StateVariableConfig)>,
}
// pub enum FeatureDependency {
//     /// names an upstream feature that provides a time value
//     /// which will be appended to the values in some destination(s)
//     TimeDependency {
//         /// name of the feature that contains a time value we will copy
//         time_feature: String,
//         /// name of feature(s) that the time value is copied to
//         destinations: Vec<StateVariableConfig>,
//     },
//     /// names an upstream feature that provides speed which can be used to compute a time value
//     /// which will be added to the existing values for [`fieldname::EDGE_TIME`] and [`fieldname::TRIP_TIME`]
//     SpeedDependency {
//         /// name of the feature that contains a speed value that will be referenced
//         speed_feature: String,
//         /// name of feature(s) that a time value, derived from the speed and edge_distance features, will be copied to
//         destinations: Vec<StateVariableConfig>,
//     },
//     /// names an upstream feature that will be copied into another location.
//     /// for example, in walk-mode trips with a penalty factor, this can be used to copy it over
//     /// to a state feature named for cost aggregation.
//     CustomFeatureCopy {
//         /// upstream feature name to copy from
//         source: String,
//         /// name of feature(s) to copy the source feature to
//         destinations: Vec<StateVariableConfig>,
//         // /// the custom feature unit type, should correspond to a [`routee_compass_core::model::state::CustomVariableConfig`]
//         // unit: DependencyUnitType,
//     },
// }

impl FeatureDependency {
    pub fn new(
        conf: &FeatureDependencyConfig,
        output_features: &HashMap<String, StateVariableConfig>,
    ) -> Result<FeatureDependency, TraversalModelError> {
        let destination_features = conf.destination_features.iter().map(|k| {
            let v = output_features.get(k).ok_or_else(|| TraversalModelError::BuildError(format!("multimodal traversal dependency declared on feature '{k}' not listed in output_features collection")))?;
            Ok((k.clone(), v.clone()))
        }).collect::<Result<Vec<_>, TraversalModelError>>()?;
        Ok(Self {
            input_feature: conf.input_feature.clone(),
            destination_features,
        })
    }

    /// maps state to mode-specific feature slots. supported operations:
    ///   - copy time feature to time feature(s)
    ///   - use speed feature to compute time and add to time feature(s)
    ///   - TODO: custom feature mappings
    pub fn apply_feature_dependency(
        &self,
        state: &mut [StateVariable],
        state_model: &StateModel,
    ) -> Result<(), StateModelError> {
        for (out_name, out_feature) in self.destination_features.iter() {
            match (&self.input_feature, out_feature) {
                (
                    InputFeature::Speed { unit, .. },
                    StateVariableConfig::Time { accumulator, .. },
                ) => {
                    let distance = state_model.get_distance(state, bambam_feature::EDGE_DISTANCE)?;
                    let speed = state_model.get_speed(state, &self.input_feature.name())?;
                    let time: Time = distance / speed;
                    if *accumulator {
                        state_model.add_time(state, out_name, &time)?;
                    } else {
                        state_model.set_time(state, out_name, &time)?;
                    }
                }
                (InputFeature::Time { .. }, StateVariableConfig::Time { accumulator, .. }) => {
                    let time = state_model.get_time(state, &self.input_feature.name())?;
                    if *accumulator {
                        state_model.add_time(state, out_name, &time)?;
                    } else {
                        state_model.set_time(state, out_name, &time)?;
                    }
                }
                (InputFeature::Custom { .. }, StateVariableConfig::Custom { value, .. }) => {
                    match value {
                        CustomVariableConfig::FloatingPoint { .. } => {
                            let value =
                                state_model.get_custom_f64(state, &self.input_feature.name())?;
                            state_model.set_custom_f64(state, out_name, &value)?;
                        }
                        CustomVariableConfig::SignedInteger { .. } => {
                            let value =
                                state_model.get_custom_i64(state, &self.input_feature.name())?;
                            state_model.set_custom_i64(state, out_name, &value)?;
                        }
                        CustomVariableConfig::UnsignedInteger { .. } => {
                            let value =
                                state_model.get_custom_u64(state, &self.input_feature.name())?;
                            state_model.set_custom_u64(state, out_name, &value)?;
                        }
                        CustomVariableConfig::Boolean { .. } => {
                            let value =
                                state_model.get_custom_bool(state, &self.input_feature.name())?;
                            state_model.set_custom_bool(state, out_name, &value)?;
                        }
                    }
                }
                _ => {
                    return Err(StateModelError::RuntimeError(format!(
                        "invalid FeatureDependency mapping from '{}'->'{}' not supported",
                        self.input_feature.name(),
                        out_name
                    )))
                }
            }
        }
        Ok(())
    }
}
