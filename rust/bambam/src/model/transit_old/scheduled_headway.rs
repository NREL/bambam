use chrono::{DateTime, Utc};
use routee_compass_core::model::unit::Time;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ScheduledHeadway {
    departure: DateTime<Utc>,
    duration: Time,
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
    pub fn dummy_comparator(t: DateTime<Utc>) -> ScheduledHeadway {
        ScheduledHeadway {
            departure: t,
            duration: Time::ONE,
        }
    }
}
