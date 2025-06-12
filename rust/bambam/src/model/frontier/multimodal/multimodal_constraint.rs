use routee_compass_core::model::unit::{Time, TimeUnit};
use std::collections::HashMap;

pub enum MultimodalConstraint {
    AllowedModes(Vec<String>),
    ModeCounts(HashMap<String, u64>),
    MaxTripLegs(u64),
    MaxTime(HashMap<String, (Time, TimeUnit)>),
}
