use super::DelayAggregationType;
use routee_compass_core::model::unit::TimeUnit;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeDelayConfig {
    pub lookup_file: String,
    pub time_unit: TimeUnit,
    pub aggregation: DelayAggregationType,
}
