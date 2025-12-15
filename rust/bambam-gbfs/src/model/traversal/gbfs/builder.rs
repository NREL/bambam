use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};

use crate::model::traversal::gbfs::{GbfsTraversalConfig, GbfsTraversalService};

pub struct GbfsTraversalBuilder {}

impl TraversalModelBuilder for GbfsTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: GbfsTraversalConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // this is where you will read GBFS files and store the data as fields
        // on the GBFS traversal service.
        let service = GbfsTraversalService::new(config);
        Ok(Arc::new(service))
    }
}
