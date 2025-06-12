use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// verbatim duplication of UnitCodecType in routee-compass-core, but adding Clone, Debug + Hash
/// (will be fixed in next version of routee-compass-core)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyUnitType {
    FloatingPoint,
    SignedInteger,
    UnsignedInteger,
    Boolean,
}

impl Display for DependencyUnitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            DependencyUnitType::FloatingPoint => String::from("floating_point"),
            DependencyUnitType::SignedInteger => String::from("signed_integer"),
            DependencyUnitType::UnsignedInteger => String::from("unsigned_integer"),
            DependencyUnitType::Boolean => String::from("boolean"),
        };
        write!(f, "{}", msg)
    }
}
