use std::sync::Arc;

use routee_compass_core::model::frontier::FrontierModel;

use super::BoardingConstraintEngine;

pub struct BoardingConstraintModel {
    pub engine: Arc<BoardingConstraintEngine>,
}

/// restricts where GBFS boarding can occur by zone
impl BoardingConstraintModel {
    pub fn new(engine: Arc<BoardingConstraintEngine>) -> BoardingConstraintModel {
        BoardingConstraintModel { engine }
    }
}

impl FrontierModel for BoardingConstraintModel {
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
