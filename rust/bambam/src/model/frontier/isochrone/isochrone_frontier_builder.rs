use routee_compass_core::model::frontier::{
    FrontierModelBuilder, FrontierModelError, FrontierModelService,
};
use std::sync::Arc;

use super::isochrone_frontier_service::IsochroneFrontierService;

pub struct IsochroneFrontierBuilder {}

impl FrontierModelBuilder for IsochroneFrontierBuilder {
    fn build(
        &self,
        _config: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        // let time_limit = config
        //     .get_config_serde::<Time>(String::from("time_limit"), String::from("isochrone"))
        //     .map_err(|e| {
        //         FrontierModelError::BuildError(format!(
        //             "failure reading time_limit from query: {}",
        //             e.to_string()
        //         ))
        //     })?;

        let model = IsochroneFrontierService {};
        Ok(Arc::new(model))
    }
}
