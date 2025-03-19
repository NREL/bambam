use std::ops::Bound;

use super::departure::Departure;
use routee_compass_core::model::unit::{Time, TimeUnit};
use skiplist::OrderedSkipList;

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
    pub fn next_departure_time(
        self,
        time: (&Time, &TimeUnit),
    ) -> Result<Option<Departure>, String> {
        let departure_query = Departure::departure_list_query(time);
        let next_departure_option = self.0.upper_bound(Bound::Included(&departure_query));
        Ok(next_departure_option.cloned())
    }
}
