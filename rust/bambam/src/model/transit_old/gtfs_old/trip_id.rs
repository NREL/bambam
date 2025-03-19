use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display};

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Default)]
pub struct TripId(pub usize);

impl PartialOrd for TripId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for TripId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Display for TripId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TripId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}
