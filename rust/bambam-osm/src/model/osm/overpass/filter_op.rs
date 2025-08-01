use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone)]
pub enum FilterOp {
    Equals,
    NotEquals,
}

impl FromStr for FilterOp {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "=" => Ok(Self::Equals),
            "~" => Ok(Self::NotEquals),
            _ => Err(format!("unknown overpass query operation '{s}'")),
        }
    }
}

impl Display for FilterOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterOp::Equals => write!(f, "="),
            FilterOp::NotEquals => write!(f, "~"),
        }
    }
}
