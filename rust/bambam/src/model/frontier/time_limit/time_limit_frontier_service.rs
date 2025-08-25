use crate::model::frontier::time_limit::{TimeLimitConfig, TimeLimitFrontierConfig};

use super::time_limit_frontier_model::TimeLimitFrontierModel;
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
    unit::TimeUnit,
};
use uom::si::f64::Time;
use std::sync::Arc;

pub struct TimeLimitFrontierService {
    time_limit: TimeLimitConfig,
}

impl TimeLimitFrontierService {
    pub fn new(conf: &TimeLimitFrontierConfig) -> TimeLimitFrontierService {
        TimeLimitFrontierService {
            time_limit: conf.time_limit.clone(),
        }
    }
}

impl FrontierModelService for TimeLimitFrontierService {
    fn build(
        &self,
        query: &serde_json::Value,
        _state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        log::debug!("begin FrontierModelService::build for TimeLimitFrontierService");
        let conf = match query.get(super::TIME_LIMIT_FIELD) {
            None => Ok(self.time_limit.clone()),
            Some(time_limit_json) => {
                let time_limit: TimeLimitConfig = serde_json::from_value(time_limit_json.clone())
                    .map_err(|e| {
                    FrontierModelError::FrontierModelError(format!(
                        "failure reading query time_limit for isochrone frontier model: {e}"
                    ))
                })?;
                Ok(time_limit)
            }
        }?;

        let time_limit = conf.time_limit()?;
        let model = TimeLimitFrontierModel { time_limit };
        Ok(Arc::new(model))
    }
}
