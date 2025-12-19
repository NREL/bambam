use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};

use super::{BoardingTraversalConfig, BoardingTraversalService};

pub struct BoardingTraversalBuilder {}

impl TraversalModelBuilder for BoardingTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: BoardingTraversalConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // this is where you will read GBFS files and store the data as fields
        // on the GBFS traversal service.
        let service = BoardingTraversalService::new(config);
        Ok(Arc::new(service))
    }
}
