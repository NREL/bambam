use std::fmt::Display;

use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::model::unit::{AsF64, DistanceUnit, EnergyUnit, TimeUnit};
use serde::{Deserialize, Serialize};

use crate::model::output_plugin::mep_score::{IntensityCategory, IntensityValueConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IntensityValue {
    /// intensity in energy per unit distance
    Energy {
        fieldname: Option<String>,
        value: f64,
    },
    Cost {
        fieldname: Option<String>,
        value: f64,
    },
    Time {
        fieldname: Option<String>,
        value: f64,
    },
}

impl From<(&IntensityCategory, &IntensityValueConfig)> for IntensityValue {
    fn from(value: (&IntensityCategory, &IntensityValueConfig)) -> Self {
        let (cat, value_conf) = value;
        let fieldname = value_conf.fieldname();
        let value = value_conf.value();
        match cat {
            IntensityCategory::Energy => Self::Energy { fieldname, value },
            IntensityCategory::Cost => Self::Cost { fieldname, value },
            IntensityCategory::Time => Self::Time { fieldname, value },
        }
    }
}

impl Display for IntensityValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IntensityValue::Energy { .. } => "energy",
            IntensityValue::Cost { .. } => "cost",
            IntensityValue::Time { .. } => "time",
        };
        write!(f, "{}", s)
    }
}

impl IntensityValue {
    const DEFAULT_ENERGY_FIELDNAME: &str = "trip_energy";
    const DEFAULT_TIME_FIELDNAME: &str = "trip_time";
    const DEFAULT_COST_FIELDNAME: &str = "trip_cost";

    /// provide the state feature name
    pub fn get_fieldname(&self) -> &str {
        match self {
            IntensityValue::Energy { fieldname, .. } => match fieldname {
                None => Self::DEFAULT_ENERGY_FIELDNAME,
                Some(f) => f,
            },
            IntensityValue::Cost { fieldname, .. } => match fieldname {
                None => Self::DEFAULT_COST_FIELDNAME,
                Some(f) => f,
            },
            IntensityValue::Time { fieldname, .. } => match fieldname {
                None => Self::DEFAULT_TIME_FIELDNAME,
                Some(f) => f,
            },
        }
    }

    /// gets the value
    pub fn get_value(&self) -> f64 {
        match self {
            IntensityValue::Energy { value, .. } => *value,
            IntensityValue::Cost { value, .. } => *value,
            IntensityValue::Time { value, .. } => *value,
        }
    }
}
