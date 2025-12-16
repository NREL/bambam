use std::sync::Arc;

use crate::model::frontier::geofence::GeofenceConstraintEngine;

use super::{GeofenceConstraintConfig, GeofenceConstraintService};
use routee_compass_core::model::frontier::{
    FrontierModelBuilder, FrontierModelError, FrontierModelService,
};
use routee_compass_core::util::geo::PolygonalRTree;
pub struct GeofenceConstraintBuilder {}

impl FrontierModelBuilder for GeofenceConstraintBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        let config: GeofenceConstraintConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let rtree = PolygonalRTree::new(vec![]).map_err(FrontierModelError::BuildError)?;
        let engine = GeofenceConstraintEngine::new(config, rtree);
        let service = GeofenceConstraintService::new(engine);
        Ok(Arc::new(service))
    }
}
