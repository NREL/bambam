use routee_compass_core::model::{frontier::FrontierModelError, unit::TimeUnit};
use serde::{Deserialize, Serialize};
use uom::si::f64::Time;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeLimitConfig {
    pub time: f64,
    pub time_unit: TimeUnit,
}

impl TimeLimitConfig {
    pub fn time_limit(&self) -> Result<Time, FrontierModelError> {
        if self.time <= 0.0 {
            Err(FrontierModelError::BuildError(format!(
                "frontier model time limit must be non-negative, found {}",
                self.time
            )))
        } else {
            Ok(self.time_unit.to_uom(self.time))
        }
    }
}
