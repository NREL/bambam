use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum IntensityValueConfig {
    ValueOnly(f64),
    ValueWithFieldname { fieldname: String, value: f64 },
}

impl IntensityValueConfig {
    pub fn fieldname(&self) -> Option<String> {
        match self {
            IntensityValueConfig::ValueOnly(_) => None,
            IntensityValueConfig::ValueWithFieldname { fieldname, .. } => Some(fieldname.clone()),
        }
    }

    pub fn value(&self) -> f64 {
        match self {
            IntensityValueConfig::ValueOnly(value) => *value,
            IntensityValueConfig::ValueWithFieldname { value, .. } => *value,
        }
    }
}
