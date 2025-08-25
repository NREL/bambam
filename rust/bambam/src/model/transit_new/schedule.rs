use super::departure::Departure;
use routee_compass_core::model::unit::TimeUnit;
use skiplist::OrderedSkipList;
use std::ops::Bound;
use uom::si::f64::Time;

pub struct Schedule(OrderedSkipList<Departure>);

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Schedule {
        todo!()
    }
    pub fn next_departure_time(self, time: &Time) -> Result<Option<Departure>, String> {
        let departure_query = Departure::query(*time);
        let next_departure_option = self.0.upper_bound(Bound::Included(&departure_query));
        Ok(next_departure_option.cloned())
    }
}
