use routee_compass_core::model::unit::{Time, TimeUnit};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeLimit {
    pub time: Time,
    pub time_unit: TimeUnit,
}
