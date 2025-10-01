use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};
use serde_json::Value;

use crate::model::{
    state::{MultimodalMapping, MultimodalStateMapping},
    traversal::multimodal::{
        MultimodalTraversalConfig, MultimodalTraversalModel, MultimodalTraversalQuery,
    },
};

pub struct MultimodalTraversalService {
    pub config: MultimodalTraversalConfig,
    pub mode_to_state: Arc<MultimodalStateMapping>,
    pub route_id_to_state: Arc<MultimodalStateMapping>,
}

impl MultimodalTraversalService {
    pub fn new(
        config: MultimodalTraversalConfig,
    ) -> Result<MultimodalTraversalService, TraversalModelError> {
        let mode_to_state = Arc::new(MultimodalMapping::new(&config.available_modes)?);
        let route_id_to_state = Arc::new(MultimodalMapping::new(&config.available_route_ids)?);
        let result = MultimodalTraversalService {
            config,
            mode_to_state,
            route_id_to_state,
        };
        Ok(result)
    }
}

impl TraversalModelService for MultimodalTraversalService {
    fn build(&self, query: &Value) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let config: MultimodalTraversalQuery = serde::Deserialize::deserialize(query).map_err(|e| TraversalModelError::BuildError(format!("failure while deserializing query in MultimodalTraversalService for {}-mode: {e}", self.config.this_mode)))?;
        let mode_to_state = match config.available_modes {
            Some(available_modes) => Arc::new(MultimodalMapping::new(&available_modes)?),
            None => self.mode_to_state.clone(),
        };
        let route_id_to_state = match config.available_route_ids {
            Some(available_route_ids) => Arc::new(MultimodalMapping::new(&available_route_ids)?),
            None => self.route_id_to_state.clone(),
        };
        let model = MultimodalTraversalModel::new(
            self.config.this_mode.clone(),
            self.config.max_trip_legs,
            mode_to_state,
            route_id_to_state,
        );
        Ok(Arc::new(model))
    }
}
