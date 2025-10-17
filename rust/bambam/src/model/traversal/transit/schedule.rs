use chrono::{Duration, Months, NaiveDateTime};
use skiplist::OrderedSkipList;
// use uom::si::f64::Time;

#[derive(Debug, Clone, Eq, Copy)]
pub struct Departure {
    pub src_departure_time: NaiveDateTime,
    pub dst_arrival_time: NaiveDateTime,
}

impl Departure {
    pub fn infinity_from(datetime: NaiveDateTime) -> Option<Self> {
        let infinity = datetime.checked_add_months(Months::new(72));

        infinity.map(|v| Self {
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

impl Ord for Departure {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.src_departure_time.cmp(&other.src_departure_time)
    }
}

pub type Schedule = OrderedSkipList<Departure>;
