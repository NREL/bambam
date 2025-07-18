use crate::model::{fieldname, frontier::isochrone::TimeLimit};
use routee_compass_core::{
    algorithm::search::{Direction, SearchTreeBranch},
    model::{
        frontier::{FrontierModel, FrontierModelError},
        network::{Edge, VertexId},
        state::{StateModel, StateVariable},
        unit::{Convert, Time, TimeUnit},
    },
};
use std::{borrow::Cow, collections::HashMap};

pub struct IsochroneFrontierModel {
    pub time_limit: TimeLimit,
}

impl FrontierModel for IsochroneFrontierModel {
    fn valid_frontier(
        &self,
        _edge: &Edge,
        state: &[StateVariable],
        _tree: &HashMap<VertexId, SearchTreeBranch>,
        _direction: &Direction,
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        let (time, time_unit) = state_model
            .get_time(
                state,
                fieldname::TRIP_TIME,
                Some(&self.time_limit.time_unit),
            )
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let mut time_cow = Cow::Owned(time);
        time_unit
            .convert(&mut time_cow, &self.time_limit.time_unit)
            .map_err(|e| {
                FrontierModelError::FrontierModelError(format!(
                    "failure converting time unit during isochrone frontier model: {}",
                    e
                ))
            })?;
        let is_valid = time_cow.as_ref() <= &self.time_limit.time;
        Ok(is_valid)
    }

    fn valid_edge(&self, _edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}
