use chrono::NaiveDateTime;
use skiplist::OrderedSkipList;
// use uom::si::f64::Time;

#[derive(Debug, Clone)]
pub struct Departure {
    pub route_id: i64,
    pub src_departure_time: NaiveDateTime,
    pub dst_arrival_time: NaiveDateTime,
}

impl PartialEq for Departure {
    fn eq(&self, other: &Self) -> bool {
        self.src_departure_time == other.src_departure_time
    }
}

impl PartialOrd for Departure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.src_departure_time
            .partial_cmp(&other.src_departure_time)
    }
}

pub type Schedule = OrderedSkipList<Departure>;
