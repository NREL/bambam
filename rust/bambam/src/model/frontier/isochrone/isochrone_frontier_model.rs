use crate::model::fieldname;
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError},
    network::Edge,
    state::{StateModel, StateVariable},
    unit::{Time, TimeUnit},
};

pub struct IsochroneFrontierModel {
    pub time_limit: Time, // assumed in same time unit as traversal model,
    pub time_unit: TimeUnit,
}

impl FrontierModel for IsochroneFrontierModel {
    fn valid_frontier(
        &self,
        _edge: &Edge,
        state: &[StateVariable],
        _tree: &std::collections::HashMap<
            routee_compass_core::model::network::VertexId,
            routee_compass_core::algorithm::search::SearchTreeBranch,
        >,
        _direction: &routee_compass_core::algorithm::search::Direction,
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        let (time, _) = state_model
            .get_time(state, fieldname::TRIP_TIME, Some(&self.time_unit))
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let is_valid = time <= self.time_limit;
        Ok(is_valid)
    }

    fn valid_edge(&self, _edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}
