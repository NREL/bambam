use std::borrow::Cow;

use crate::model::fieldname;
use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError},
    network::Edge,
    state::{StateModel, StateVariable},
    unit::{Convert, Time, TimeUnit},
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
        let (time, time_unit) = state_model
            .get_time(state, fieldname::TRIP_TIME, Some(&self.time_unit))
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let mut time_cow = Cow::Owned(time);
        time_unit
            .convert(&mut time_cow, &self.time_unit)
            .map_err(|e| {
                FrontierModelError::FrontierModelError(format!(
                    "failure converting time unit during isochrone frontier model: {}",
                    e
                ))
            })?;
        let is_valid = time_cow.as_ref() <= &self.time_limit;
        Ok(is_valid)
    }

    fn valid_edge(&self, _edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}
