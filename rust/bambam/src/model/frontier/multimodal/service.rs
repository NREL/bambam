use std::sync::Arc;

use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
};

use crate::model::{
    frontier::multimodal::{
        model::MultimodalFrontierModel, MultimodalFrontierConfig, MultimodalFrontierConstraint,
        MultimodalFrontierEngine,
    },
    state::{MultimodalMapping, MultimodalStateMapping},
};

pub struct MultimodalFrontierService {
    pub engine: Arc<MultimodalFrontierEngine>,
}

impl MultimodalFrontierService {
    pub fn new(
        config: MultimodalFrontierConfig,
    ) -> Result<MultimodalFrontierService, FrontierModelError> {
        let mode_mapping = MultimodalMapping::new(&config.available_modes).map_err(|e| {
            FrontierModelError::BuildError(format!("while building mode mapping: {e}"))
        })?;
        let route_id_mapping =
            MultimodalMapping::new(&config.available_route_ids).map_err(|e| {
                FrontierModelError::BuildError(format!("while building route_id mapping: {e}"))
            })?;
        let mode_to_state = Arc::new(mode_mapping);
        let route_id_to_state = Arc::new(route_id_mapping);
        let constraints = config
            .constraints
            .iter()
            .map(MultimodalFrontierConstraint::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let engine = MultimodalFrontierEngine {
            mode: config.mode,
            constraints,
            max_trip_legs: config.max_trip_legs,
            mode_to_state,
            route_id_to_state,
            use_route_ids: config.use_route_ids,
        };
        let service = MultimodalFrontierService {
            engine: Arc::new(engine),
        };
        Ok(service)
    }
}

impl FrontierModelService for MultimodalFrontierService {
    fn build(
        &self,
        query: &serde_json::Value,
        state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        let model = MultimodalFrontierModel::new(self.engine.clone());
        Ok(Arc::new(model))
    }
}
