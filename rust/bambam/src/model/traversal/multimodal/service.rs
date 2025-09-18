use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};
use serde_json::Value;

use crate::model::{
    state::MultimodalMapping,
    traversal::multimodal::{
        MultimodalTraversalConfig, MultimodalTraversalModel, MultimodalTraversalQuery,
    },
};

pub struct MultimodalTraversalService {
    pub config: MultimodalTraversalConfig,
    pub mode_mapping: Arc<MultimodalMapping<String, i64>>,
}

impl MultimodalTraversalService {
    pub fn new(
        config: MultimodalTraversalConfig,
    ) -> Result<MultimodalTraversalService, TraversalModelError> {
        let mode_mapping = Arc::new(MultimodalMapping::new(&config.available_modes)?);
        let result = MultimodalTraversalService {
            config,
            mode_mapping,
        };
        Ok(result)
    }
}

impl TraversalModelService for MultimodalTraversalService {
    fn build(&self, query: &Value) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let config: MultimodalTraversalQuery = serde::Deserialize::deserialize(query).map_err(|e| TraversalModelError::BuildError(format!("failure while deserializing query in MultimodalTraversalService for {}-mode: {e}", self.config.this_mode)))?;
        let mode_mapping = match config.available_modes {
            Some(available_modes) => Arc::new(MultimodalMapping::new(&available_modes)?),
            None => self.mode_mapping.clone(),
        };
        let model = MultimodalTraversalModel::new(
            self.config.this_mode.clone(),
            self.config.max_trip_legs,
            mode_mapping,
        );
        Ok(Arc::new(model))
    }
}
