use super::isochrone_frontier_model::IsochroneFrontierModel;
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
    unit::{Time, TimeUnit},
};
use std::sync::Arc;

pub struct IsochroneFrontierService {}

impl FrontierModelService for IsochroneFrontierService {
    fn build(
        &self,
        query: &serde_json::Value,
        _state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        log::debug!("begin FrontierModelService::build for IsochroneFrontierService");
        let time_limit = query
            .get_config_serde::<Time>(&String::from("time_limit"), &String::from("isochrone"))
            .map_err(|e| {
                FrontierModelError::BuildError(format!(
                    "failure reading time_limit from query: {e}"
                ))
            })?;
        let time_unit = query
            .get_config_serde_optional::<TimeUnit>(
                &String::from("time_unit"),
                &String::from("isochrone"),
            )
            .map_err(|e| {
                FrontierModelError::BuildError(format!("failure reading time_unit from query: {e}"))
            })?
            .unwrap_or(TimeUnit::Minutes);

        let model = IsochroneFrontierModel {
            time_limit,
            time_unit,
        };
        Ok(Arc::new(model))
    }
}
