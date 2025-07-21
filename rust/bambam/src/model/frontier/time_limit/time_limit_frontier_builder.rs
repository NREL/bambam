use crate::model::frontier::time_limit::{TimeLimitConfig, TimeLimitFrontierConfig};

use super::time_limit_frontier_service::TimeLimitFrontierService;
use routee_compass_core::model::{
    frontier::{FrontierModelBuilder, FrontierModelError, FrontierModelService},
    unit::{Time, TimeUnit},
};
use std::sync::Arc;

pub struct TimeLimitFrontierBuilder {}

impl FrontierModelBuilder for TimeLimitFrontierBuilder {
    fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        let conf: TimeLimitFrontierConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                FrontierModelError::BuildError(format!(
                    "failure reading isochrone frontier model configuration: {}",
                    e
                ))
            })?;
        let model = TimeLimitFrontierService::new(&conf);
        Ok(Arc::new(model))
    }
}
