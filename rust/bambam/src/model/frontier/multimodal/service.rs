use std::{path::Path, sync::Arc};

use routee_compass_core::{
    model::{
        frontier::{FrontierModel, FrontierModelError, FrontierModelService},
        state::StateModel,
    },
    util::fs::{read_decoders, read_utils},
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
        let route_id_to_state = match &config.route_ids_input_file {
            Some(input_file) => {
                let rmap =
                    MultimodalStateMapping::from_enumerated_category_file(Path::new(&input_file))
                        .map_err(|e| {
                        FrontierModelError::BuildError(format!(
                            "failure building route id mapping from input file {input_file}: {e}"
                        ))
                    })?;
                Arc::new(Some(rmap))
            }
            None => Arc::new(None),
        };
        let mode_to_state = Arc::new(mode_mapping);
        let constraints = config
            .constraints
            .iter()
            .map(MultimodalFrontierConstraint::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let engine = MultimodalFrontierEngine {
            mode: config.this_mode,
            constraints,
            max_trip_legs: config.max_trip_legs,
            mode_to_state,
            route_id_to_state,
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
