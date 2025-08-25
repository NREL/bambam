use routee_compass_core::model::unit::{TimeUnit};
use uom::si::f64::Time;
use std::collections::HashMap;

pub enum MultimodalConstraint {
    AllowedModes(Vec<String>),
    ModeCounts(HashMap<String, u64>),
    MaxTripLegs(u64),
    MaxTime(HashMap<String, Time>),
}
