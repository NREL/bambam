use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// a row in the schedules CSV file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleRow {
    pub edge_id: usize,
    pub src_departure_time: NaiveDateTime,
    pub dst_arrival_time: NaiveDateTime,
    pub route_id: String,
}
