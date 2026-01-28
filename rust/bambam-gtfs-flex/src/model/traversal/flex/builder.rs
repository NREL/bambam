use std::sync::Arc;

use routee_compass_core::model::traversal::{TraversalModelBuilder, TraversalModelError};

use crate::model::traversal::flex::{
    service::GtfsFlexTraversalService, GtfsFlexTraversalConfig,
    GtfsFlexTraversalEngine,
};

pub struct GtfsFlexTraversalBuilder {}

impl TraversalModelBuilder for GtfsFlexTraversalBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<
        std::sync::Arc<dyn routee_compass_core::model::traversal::TraversalModelService>,
        routee_compass_core::model::traversal::TraversalModelError,
    > {
        let config: GtfsFlexTraversalConfig =
            serde_json::from_value(parameters.clone()).map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failed reading GtfsFlexTraversalConfig from config: {e}"
                ))
            })?;
        let engine = GtfsFlexTraversalEngine::try_from(&config)?;
        let service = GtfsFlexTraversalService::new(engine);
        Ok(Arc::new(service))
    }
}
