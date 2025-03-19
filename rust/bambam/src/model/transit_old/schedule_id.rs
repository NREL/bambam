use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display};

/// an identifier for a given schedule.
/// in GTFS, this represents a given agency/route/trip instance
#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Default)]
pub struct ScheduleId(pub usize);

impl PartialOrd for ScheduleId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for ScheduleId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Display for ScheduleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ScheduleId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}
