use std::borrow::Cow;

use routee_compass_core::model::unit::{AsF64, TimeUnit, UnitError};
use uom::{si::f64::Time, ConstZero};

/// represents a single departure time for a static scheduled route.
#[derive(Clone, Debug)]
pub struct Departure {
    departure_time: Time, // >> 24 hours
    leg_duration: Time,
}

impl Departure {
    pub fn new(departure_time: Time, leg_duration: Time) -> Departure {
        Departure {
            departure_time,
            leg_duration,
        }
    }

    /// creates a query into an OrderedSkipList<Departure>
    /// this creates a 'dummy' value with the matching departure time.
    /// OrderedSkipList.upper_bound() query value must be of type `Departure`.
    pub fn query(departure_time: Time) -> Departure {
        Departure {
            departure_time,
            leg_duration: Time::ZERO,
        }
    }
}

impl PartialEq for Departure {
    fn eq(&self, other: &Self) -> bool {
        self.departure_time == other.departure_time
    }
}

impl PartialOrd for Departure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.departure_time.partial_cmp(&other.departure_time)
    }
}
