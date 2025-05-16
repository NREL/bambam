use super::traversal::time_delay::TimeDelayLookup;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{OutputFeature, StateModel, StateModelError, StateVariable},
    traversal::TraversalModelError,
    unit::{Distance, DistanceUnit, Speed, SpeedUnit, Time, TimeUnit},
};

/// provides field names for all MEP traversal models
pub mod field {
    pub const DISTANCE: &str = "distance";

    // should match expectations of core lib feature name
    pub const TRAVERSAL_TIME: &str = "time";

    pub const ARRIVAL_DELAY: &str = "delay";
    pub const ROUTE_ID: &str = "route_id";
    pub const EMPTY_ROUTE_ID: i64 = -1;
}

/// the default set of state features used by different MEP traversal
/// models, where time is assumed in minutes and distance in miles.
pub fn default_state_features() -> Vec<(String, OutputFeature)> {
    vec![
        (
            String::from(field::ARRIVAL_DELAY),
            OutputFeature::Time {
                time_unit: TimeUnit::Minutes,
                initial: Time::ZERO,
                accumulator: false,
            },
        ),
        (
            String::from(field::TRAVERSAL_TIME),
            OutputFeature::Time {
                time_unit: TimeUnit::Minutes,
                initial: Time::ZERO,
                accumulator: true,
            },
        ),
        (
            String::from(field::DISTANCE),
            OutputFeature::Distance {
                distance_unit: DistanceUnit::Miles,
                initial: Distance::ZERO,
                accumulator: true,
            },
        ),
    ]
}

/// helper that combines the arrival delay with the traversal time to produce
/// the time to reach this point and call it a destination.
pub fn get_reachability_time_minutes(
    state: &[StateVariable],
    state_model: &StateModel,
) -> Result<Time, StateModelError> {
    let (traversal, _) =
        state_model.get_time(state, field::TRAVERSAL_TIME, Some(&TimeUnit::Minutes))?;
    let (arrival, _) =
        state_model.get_time(state, field::ARRIVAL_DELAY, Some(&TimeUnit::Minutes))?;

    Ok(traversal + arrival)
}

/// completes a default MEP traversal, which includes assignment of:
///   - a departure delay, if applicable
///   - a distance and time state update due to edge traversal at the given speed
///   - an arrival delay to this state instance
///
/// # Arguments
/// * `trajectory`      - graph trajectory for this traversal
/// * `edge_speed`      - speed to use when computing time
/// * `state`           - state to update, an in-out argument
/// * `state_model`     - API for updating the state
/// * `departure_delay` - optional departure delay table
/// * `arrival_delay`   - optional arrival delay table
pub fn default_mep_traversal(
    trajectory: (&Vertex, &Edge, &Vertex),
    edge_speed: (&Speed, &SpeedUnit),
    state: &mut [StateVariable],
    state_model: &StateModel,
    departure_delay: &Option<TimeDelayLookup>,
    arrival_delay: &Option<TimeDelayLookup>,
) -> Result<(), TraversalModelError> {
    let (src, edge, dst) = trajectory;
    let (speed, speed_unit) = edge_speed;
    assign_departure_delay(src, edge, state, state_model, departure_delay)?;

    state_model.add_distance(
        state,
        field::DISTANCE,
        &edge.distance,
        &DistanceUnit::Meters,
    )?;

    let (traversal_time, time_unit) =
        Time::create((&edge.distance, &DistanceUnit::Meters), (speed, speed_unit))?;
    state_model.add_time(state, field::TRAVERSAL_TIME, &traversal_time, &time_unit)?;

    assign_arrival_delay(dst, state, state_model, arrival_delay)?;

    Ok(())
}

/// assign the delay cost for arriving at this link.
pub fn assign_arrival_delay(
    dst: &Vertex,
    state: &mut [StateVariable],
    state_model: &StateModel,
    delay_lookup: &Option<TimeDelayLookup>,
) -> Result<(), TraversalModelError> {
    if let Some((delay, tu)) = get_delay(dst, delay_lookup) {
        state_model.add_time(state, field::TRAVERSAL_TIME, &delay, tu)?
    };
    Ok(())
}

/// if the trip is just beginning, it has a distance state of zero. we use this
/// fact along with an optional lookup function to assign a departure trip delay,
/// such as a TNC wait time or departure parking delay.
pub fn assign_departure_delay(
    src: &Vertex,
    _edge: &Edge,
    state: &mut [StateVariable],
    state_model: &StateModel,
    delay_lookup: &Option<TimeDelayLookup>,
) -> Result<(), TraversalModelError> {
    let (initial_distance, _) =
        state_model.get_distance(state, field::DISTANCE, Some(&DistanceUnit::Meters))?;
    if initial_distance == Distance::ZERO {
        if let Some((delay, tu)) = get_delay(src, delay_lookup) {
            state_model.add_time(state, field::TRAVERSAL_TIME, &delay, tu)?;
        }
    }
    Ok(())
}

pub fn get_delay<'a>(
    lookup_vertex: &Vertex,
    delay_lookup: &'a Option<TimeDelayLookup>,
) -> Option<(Time, &'a TimeUnit)> {
    if let Some(lookup) = delay_lookup {
        let g = geo::Geometry::Point(geo::Point(lookup_vertex.coordinate.0));
        lookup.find_first_delay(&g)
    } else {
        None
    }
}
