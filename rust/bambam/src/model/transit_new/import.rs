use routee_compass_core::model::{network::EdgeId};
use uom::si::f64::Time;

pub struct GtfsImporter {}

/// represents
pub struct ScheduledTraversalAttributes {
    edge_id: EdgeId,
    /// 65k route names per agency (next step down is 256 names per agency, that's too small)
    /// this assumes a metadata file mapping route_id integers to actual names
    route_id: u16,
    departure_time_seconds: Time,
    leg_duration_seconds: Time,
    // allows_bike: bool
    // allows_wheelchair: bool
}
