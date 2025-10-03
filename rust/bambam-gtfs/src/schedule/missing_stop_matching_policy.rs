use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Enumerates alternative ways to handle
/// missing lon,lat data for a stop
#[derive(Serialize, Deserialize, Debug, ValueEnum, Clone)]
pub enum MissingStopLocationPolicy {
    Fail,
    DropStop,
}
