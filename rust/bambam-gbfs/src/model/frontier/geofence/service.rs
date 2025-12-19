use std::sync::Arc;

use crate::model::frontier::geofence::{GeofenceConstraintEngine, GeofenceConstraintModel};

use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
};

pub struct GeofenceConstraintService {
    pub engine: Arc<GeofenceConstraintEngine>,
}

impl GeofenceConstraintService {
    pub fn new(engine: GeofenceConstraintEngine) -> GeofenceConstraintService {
        GeofenceConstraintService {
            engine: Arc::new(engine),
        }
    }
}

impl FrontierModelService for GeofenceConstraintService {
    fn build(
        &self,
        _query: &serde_json::Value,
        _state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        Ok(Arc::new(GeofenceConstraintModel::new(self.engine.clone())))
    }
}
