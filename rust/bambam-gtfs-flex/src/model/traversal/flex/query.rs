use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// query-time
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GtfsFlexServiceTypeTwoQuery {
    /// start time of the trip. used in conjunction with the source zone
    /// to determine the valid destination zones.
    pub start_time: NaiveDateTime,
}
