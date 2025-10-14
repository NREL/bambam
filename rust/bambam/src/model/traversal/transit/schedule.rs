use chrono::{Duration, Months, NaiveDateTime};
use skiplist::OrderedSkipList;
// use uom::si::f64::Time;

#[derive(Debug, Clone)]
pub struct Departure {
    pub route_id: i64,
    pub src_departure_time: NaiveDateTime,
    pub dst_arrival_time: NaiveDateTime,
}

impl Departure {
    pub fn infinity_from(datetime: NaiveDateTime) -> Option<Self> {
        let infinity = datetime.checked_add_months(Months::new(72));

        infinity.map(|v| Self {
            route_id: 0,
            src_departure_time: v,
            dst_arrival_time: v,
        })
    }
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
