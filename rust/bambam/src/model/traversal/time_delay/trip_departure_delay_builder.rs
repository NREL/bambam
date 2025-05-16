use crate::model::traversal::time_delay::{
    TimeDelayConfig, TimeDelayLookup, TripDepartureDelayModel,
};
use routee_compass_core::model::traversal::{
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

pub struct TripDepartureDelayBuilder {}

impl TraversalModelBuilder for TripDepartureDelayBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config: TimeDelayConfig = serde_json::from_value(parameters.clone()).map_err(|e| {
            TraversalModelError::BuildError(format!("failed to build departure delay model: {}", e))
        })?;
        let lookup = Arc::new(TimeDelayLookup::try_from(config)?);
        let service = Arc::new(TripDepartureDelayModel::new(lookup));
        Ok(service)
    }
}
