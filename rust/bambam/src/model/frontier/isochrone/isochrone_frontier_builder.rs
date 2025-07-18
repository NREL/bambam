use crate::model::frontier::isochrone::{IsochroneFrontierConfig, TimeLimit};

use super::isochrone_frontier_service::IsochroneFrontierService;
use routee_compass_core::model::{
    frontier::{FrontierModelBuilder, FrontierModelError, FrontierModelService},
    unit::{Time, TimeUnit},
};
use std::sync::Arc;

pub struct IsochroneFrontierBuilder {}

impl FrontierModelBuilder for IsochroneFrontierBuilder {
    fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        let conf: IsochroneFrontierConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                FrontierModelError::BuildError(format!(
                    "failure reading isochrone frontier model configuration: {}",
                    e
                ))
            })?;
        let model = IsochroneFrontierService::new(&conf);
        Ok(Arc::new(model))
    }
}
