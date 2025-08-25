use routee_compass_core::model::unit::TimeUnit;
use std::collections::HashMap;
use uom::si::f64::Time;

pub enum MultimodalConstraint {
    AllowedModes(Vec<String>),
    ModeCounts(HashMap<String, u64>),
    MaxTripLegs(u64),
    MaxTime(HashMap<String, Time>),
}
