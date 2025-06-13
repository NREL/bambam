use super::DelayAggregationType;
use routee_compass_core::model::unit::TimeUnit;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeDelayConfig {
    /// file containing time delays and geometries
    pub lookup_file: String,
    /// unit for time values in the file
    pub time_unit: TimeUnit,
    /// aggregation method for cases when more than one geometry intersects
    /// a road network element.
    pub aggregation: DelayAggregationType,
}
