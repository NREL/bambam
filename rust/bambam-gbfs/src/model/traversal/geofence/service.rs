use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};

use crate::model::traversal::geofence::GeofenceTraversalConfig;

pub struct GeofenceTraversalService {
    pub config: GeofenceTraversalConfig,
}

impl GeofenceTraversalService {
    pub fn new(config: GeofenceTraversalConfig) -> GeofenceTraversalService {
        GeofenceTraversalService { config }
    }
}

impl TraversalModelService for GeofenceTraversalService {
    fn build(
        &self,
        _query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        // if there's anything that can change between the execution of each search,
        // we should attempt to pull it from the query here.
        todo!()
    }
}
