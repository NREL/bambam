use chrono::NaiveDateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use uom::si::f64::Time;

pub const APP_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn deserialize_naive_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let date_str: String = String::deserialize(deserializer)?;
    chrono::NaiveDateTime::parse_from_str(&date_str, APP_DATETIME_FORMAT)
        .map_err(|e| D::Error::custom(format!("Invalid datetime format: {}", e)))
}

#[derive(Serialize, Deserialize)]
pub struct TransitTraversalQuery {
    #[serde(deserialize_with = "deserialize_naive_datetime")]
    pub start_datetime: NaiveDateTime, // Fix deserialization
    /// If true, we maintain a DWELL_TIME state variable
    pub record_dwell_time: Option<bool>,
}
