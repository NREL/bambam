use chrono::NaiveDateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use uom::si::f64::Time;

#[derive(Serialize, Deserialize)]
pub struct TransitTraversalQuery {
    pub start_datetime: NaiveDateTime,
    pub record_dwell_time: bool
}
