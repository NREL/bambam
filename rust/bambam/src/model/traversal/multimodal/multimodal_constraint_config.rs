use routee_compass_core::model::unit::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MultimodalConstraintConfig {
    AllowedModes(Vec<String>),
    ModeCounts(HashMap<String, NonZeroU64>),
    TripLegCount(NonZeroU64),
    DistanceLimit(HashMap<String, DistanceTuple>),
    TimeLimit(HashMap<String, TimeTuple>),
    EnergyLimit(HashMap<String, EnergyTuple>),
    ExactSequences(Vec<Vec<String>>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistanceTuple {
    value: Distance,
    unit: DistanceUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeTuple {
    value: Time,
    unit: TimeUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnergyTuple {
    value: Energy,
    unit: EnergyUnit,
}
