use routee_compass_core::model::unit::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MultimodalFrontierConstraintConfig {
    AllowedModes(Vec<String>),
    ModeCounts(HashMap<String, NonZeroU64>),
    TripLegCount(NonZeroU64),
    ExactSequences(Vec<Vec<String>>),
    // TODO: these metric-based constraints need to be run in the traversal model where those
    // metrics are being evaluated. but in order to produce "is invalid" they would need to
    // be able to set the Cost to infinity. perhaps by setting Time::MAX?
    // DistanceLimit(HashMap<String, DistanceTuple>),
    // TimeLimit(HashMap<String, TimeTuple>),
    // EnergyLimit(HashMap<String, EnergyTuple>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistanceTuple {
    value: f64,
    unit: DistanceUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeTuple {
    value: f64,
    unit: TimeUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnergyTuple {
    value: f64,
    unit: EnergyUnit,
}
