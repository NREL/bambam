use std::sync::Arc;

use routee_compass_core::model::access::{AccessModel, AccessModelError, AccessModelService};
use serde_json::Value;

use crate::model::{
    access::multimodal::{MultimodalAccessConfig, MultimodalAccessModel, MultimodalAccessQuery},
    state::MultimodalMapping,
};

pub struct MultimodalAccessService {
    pub config: MultimodalAccessConfig,
    pub mode_mapping: Arc<MultimodalMapping<String, i64>>,
}

impl MultimodalAccessService {
    pub fn new(
        config: MultimodalAccessConfig,
    ) -> Result<MultimodalAccessService, AccessModelError> {
        let mode_mapping = Arc::new(MultimodalMapping::new(&config.available_modes)?);
        let result = MultimodalAccessService {
            config,
            mode_mapping,
        };
        Ok(result)
    }
}

impl AccessModelService for MultimodalAccessService {
    fn build(&self, query: &Value) -> Result<Arc<dyn AccessModel>, AccessModelError> {
        let config: MultimodalAccessQuery =
            serde::Deserialize::deserialize(query).map_err(|e| {
                AccessModelError::BuildError(format!(
                    "failure while deserializing query in MultimodalAccessService for {}-mode: {e}",
                    self.config.this_mode
                ))
            })?;
        let mode_mapping = match config.available_modes {
            Some(available_modes) => Arc::new(MultimodalMapping::new(&available_modes)?),
            None => self.mode_mapping.clone(),
        };
        let model = MultimodalAccessModel::new(
            self.config.this_mode.clone(),
            self.config.max_trip_legs,
            mode_mapping,
        );
        Ok(Arc::new(model))
    }
}
