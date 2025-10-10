use chrono::NaiveDateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use uom::si::f64::Time;

#[derive(Serialize, Deserialize)]
pub struct TransitTraversalQuery {
    pub start_datetime: NaiveDateTime,
    /// If true, we maintain a DWELL_TIME state variable
    pub record_dwell_time: Option<bool>,
}
