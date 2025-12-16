use std::sync::Arc;

use routee_compass_core::model::frontier::FrontierModel;

use crate::model::frontier::geofence::GeofenceConstraintEngine;

pub struct GeofenceConstraintModel {
    pub engine: Arc<GeofenceConstraintEngine>,
}

impl GeofenceConstraintModel {
    pub fn new(engine: Arc<GeofenceConstraintEngine>) -> GeofenceConstraintModel {
        GeofenceConstraintModel { engine }
    }
}

impl FrontierModel for GeofenceConstraintModel {
    fn valid_frontier(
        &self,
        _edge: &routee_compass_core::model::network::Edge,
        _previous_edge: Option<&routee_compass_core::model::network::Edge>,
        _state: &[routee_compass_core::model::state::StateVariable],
        _state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<bool, routee_compass_core::model::frontier::FrontierModelError> {
        todo!()
    }

    fn valid_edge(
        &self,
        _edge: &routee_compass_core::model::network::Edge,
    ) -> Result<bool, routee_compass_core::model::frontier::FrontierModelError> {
        todo!()
    }
}
