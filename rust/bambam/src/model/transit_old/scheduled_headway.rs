use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use uom::{si::f64::Time, ConstZero};

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct ScheduledHeadway {
    departure: DateTime<Utc>,
    duration: Time,
}

impl Eq for ScheduledHeadway {}

impl PartialEq for ScheduledHeadway {
    fn eq(&self, other: &Self) -> bool {
        self.departure == other.departure
    }
}

impl PartialOrd for ScheduledHeadway {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.departure.cmp(&other.departure))
    }
}

impl Ord for ScheduledHeadway {
    fn cmp(&self, other: &Self) -> Ordering {
        self.departure.cmp(&other.departure)
    }
}

impl ScheduledHeadway {
    /// query an OrderedSkipList<ScheduledHeadway> with a given station arrival time.
    pub fn query(t: DateTime<Utc>) -> ScheduledHeadway {
        ScheduledHeadway {
            departure: t,
            duration: Time::ZERO,
        }
    }
}
