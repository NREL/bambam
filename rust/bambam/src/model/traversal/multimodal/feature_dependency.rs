use crate::model::{fieldname, traversal::multimodal::DependencyUnitType};
use routee_compass_core::model::{
    state::{InputFeature, StateModel, StateModelError, StateVariable},
    unit::Time,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub enum FeatureDependency {
    /// names an upstream feature that provides a time value
    /// which will be added to the existing values for [`fieldname::EDGE_TIME`] and [`fieldname::TRIP_TIME`]
    TimeDependency { time_feature: String },
    /// names an upstream feature that provides speed which can be used to compute a time value
    /// which will be added to the existing values for [`fieldname::EDGE_TIME`] and [`fieldname::TRIP_TIME`]
    SpeedDependency { speed_feature: String },
    /// names an upstream feature that will be copied into another location.
    /// for example, in walk-mode trips with a penalty factor, this can be used to copy it over
    /// to a state feature named for cost aggregation.
    CustomFeatureCopy {
        source: String,
        destination: String,
        unit: DependencyUnitType,
    },
}

impl FeatureDependency {
    pub fn as_input_feature(&self) -> (String, InputFeature) {
        match self {
            FeatureDependency::TimeDependency { time_feature } => {
                (time_feature.clone(), InputFeature::Time(None))
            }
            FeatureDependency::SpeedDependency { speed_feature } => {
                (speed_feature.clone(), InputFeature::Speed(None))
            }
            FeatureDependency::CustomFeatureCopy {
                source,
                destination,
                unit,
            } => (
                source.clone(),
                InputFeature::Custom {
                    name: source.clone(),
                    unit: unit.to_string(),
                },
            ),
        }
    }

    /// updates the state vector based on the referenced feature
    pub fn apply_feature_dependency(
        &self,
        state: &mut [StateVariable],
        state_model: &StateModel,
    ) -> Result<(), StateModelError> {
        // get the time value based on the feature dependency
        match self {
            FeatureDependency::TimeDependency { time_feature } => {
                let (time, time_unit) = state_model.get_time(state, time_feature, None)?;
                state_model.add_time(state, fieldname::EDGE_TIME, &time, &time_unit)?;
                state_model.add_time(state, fieldname::TRIP_TIME, &time, &time_unit)?;
                Ok(())
            }
            FeatureDependency::SpeedDependency { speed_feature } => {
                let (distance, distance_unit) =
                    state_model.get_distance(state, fieldname::EDGE_DISTANCE, None)?;
                let (speed, speed_unit) = state_model.get_speed(state, speed_feature, None)?;
                let (time, time_unit) =
                    Time::create((&distance, distance_unit), (&speed, speed_unit))?;
                state_model.add_time(state, fieldname::EDGE_TIME, &time, &time_unit)?;
                state_model.add_time(state, fieldname::TRIP_TIME, &time, &time_unit)?;
                Ok(())
            }
            FeatureDependency::CustomFeatureCopy {
                source,
                destination,
                unit,
            } => match unit {
                DependencyUnitType::FloatingPoint => {
                    let value = state_model.get_custom_f64(state, source)?;
                    state_model.set_custom_f64(state, destination, &value)?;
                    Ok(())
                }
                DependencyUnitType::SignedInteger => {
                    let value = state_model.get_custom_i64(state, source)?;
                    state_model.set_custom_i64(state, destination, &value)?;
                    Ok(())
                }
                DependencyUnitType::UnsignedInteger => {
                    let value = state_model.get_custom_f64(state, source)?;
                    state_model.set_custom_f64(state, destination, &value)?;
                    Ok(())
                }
                DependencyUnitType::Boolean => {
                    let value = state_model.get_custom_bool(state, source)?;
                    state_model.set_custom_bool(state, destination, &value)?;
                    Ok(())
                }
            },
        }
    }
}
