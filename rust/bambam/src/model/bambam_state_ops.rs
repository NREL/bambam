use crate::model::fieldname;

use super::traversal::time_delay::TimeDelayLookup;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{OutputFeature, StateModel, StateModelError, StateVariable},
    traversal::TraversalModelError,
    unit::{Distance, DistanceUnit, Speed, SpeedUnit, Time, TimeUnit},
};

/// helper that combines the arrival delay with the traversal time to produce
/// the time to reach this point and call it a destination.
pub fn get_reachability_time_minutes(
    state: &[StateVariable],
    state_model: &StateModel,
) -> Result<Time, StateModelError> {
    let (mut time, _) =
        state_model.get_time(state, fieldname::TRIP_TIME, Some(&TimeUnit::Minutes))?;
    if state_model.contains_key(&fieldname::TRIP_ARRIVAL_DELAY.to_string()) {
        let (arrival, _) = state_model.get_time(
            state,
            fieldname::TRIP_ARRIVAL_DELAY,
            Some(&TimeUnit::Minutes),
        )?;
        time = time + arrival;
    }

    Ok(time)
}
