use std::fmt::Display;

use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::model::unit::{AsF64, DistanceUnit, EnergyUnit, TimeUnit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntensityCategory {
    /// intensity in energy per unit distance
    Energy,
    Cost,
    Time,
}

impl Display for IntensityCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IntensityCategory::Energy => "energy",
            IntensityCategory::Cost => "cost",
            IntensityCategory::Time => "time",
        };
        write!(f, "{}", s)
    }
}
