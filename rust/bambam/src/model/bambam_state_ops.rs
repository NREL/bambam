use super::traversal::time_delay::TimeDelayLookup;
use crate::model::fieldname;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{StateModel, StateModelError, StateVariable, StateVariableConfig},
    traversal::TraversalModelError,
    unit::{DistanceUnit, SpeedUnit, TimeUnit},
};
use uom::{si::f64::Time, ConstZero};

/// helper that combines the arrival delay with the traversal time to produce
/// the time to reach this point and call it a destination.
pub fn get_reachability_time(
    state: &[StateVariable],
    state_model: &StateModel,
) -> Result<Time, StateModelError> {
    let trip_time = state_model.get_time(state, fieldname::TRIP_TIME)?;
    let has_delay = state_model.contains_key(&fieldname::TRIP_ARRIVAL_DELAY.to_string());
    let arrival_delay = if has_delay {
        state_model.get_time(state, fieldname::TRIP_ARRIVAL_DELAY)?
    } else {
        Time::ZERO
    };
    Ok(trip_time + arrival_delay)
}
