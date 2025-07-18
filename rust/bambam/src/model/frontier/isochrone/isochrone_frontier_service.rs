use crate::model::frontier::isochrone::{IsochroneFrontierConfig, TimeLimit};

use super::isochrone_frontier_model::IsochroneFrontierModel;
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
    unit::{Time, TimeUnit},
};
use std::sync::Arc;

pub struct IsochroneFrontierService {
    time_limit: TimeLimit,
}

impl IsochroneFrontierService {
    pub fn new(conf: &IsochroneFrontierConfig) -> IsochroneFrontierService {
        IsochroneFrontierService {
            time_limit: conf.time_limit.clone(),
        }
    }
}

impl FrontierModelService for IsochroneFrontierService {
    fn build(
        &self,
        query: &serde_json::Value,
        _state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        log::debug!("begin FrontierModelService::build for IsochroneFrontierService");
        let time_limit = match query.get(super::TIME_LIMIT_FIELD) {
            None => Ok(self.time_limit.clone()),
            Some(time_limit_json) => {
                let time_limit: TimeLimit = serde_json::from_value(time_limit_json.clone())
                    .map_err(|e| {
                        FrontierModelError::FrontierModelError(format!(
                            "failure reading query time_limit for isochrone frontier model: {}",
                            e
                        ))
                    })?;
                Ok(time_limit)
            }
        }?;

        let model = IsochroneFrontierModel { time_limit };
        Ok(Arc::new(model))
    }
}
