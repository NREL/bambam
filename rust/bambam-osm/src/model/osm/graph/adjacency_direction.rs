use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacencyDirection {
    Forward,
    Reverse,
}

impl Display for AdjacencyDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdjacencyDirection::Forward => write!(f, "forward"),
            AdjacencyDirection::Reverse => write!(f, "reverse"),
        }
    }
}
