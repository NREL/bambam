/// time delays accumulated throughout the trip
pub const TRIP_ENROUTE_DELAY: &str = "trip_enroute_delay";

/// time delays on arriving at a destination, such as parking, which
/// are not incorporated into the search cost function.
pub const TRIP_ARRIVAL_DELAY: &str = "trip_arrival_delay";

/// used to penalize an edge. convention is to design this
/// as one of the vehicle cost rates, via a "raw" interpretation
/// (no cost conversion) and then to use "mul" (multiplicitive)
/// cost aggregation with this value and the total edge time.
/// when this value is 1.0, no penalty is applied.
/// if it is < 1, it reduces cost, and > 1, increases cost.
pub const COST_PENALTY_FACTOR: &str = "penalty_factor";

pub use routee_compass_core::model::traversal::default::fieldname::*;
