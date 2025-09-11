use crate::model::{bambam_feature, frontier::time_limit::TimeLimitConfig};
use routee_compass_core::{
    algorithm::search::{Direction, SearchTreeBranch},
    model::{
        frontier::{FrontierModel, FrontierModelError},
        network::{Edge, VertexId},
        state::{StateModel, StateVariable},
        unit::TimeUnit,
    },
};
use std::{borrow::Cow, collections::HashMap};
use uom::si::f64::Time;

pub struct TimeLimitFrontierModel {
    pub time_limit: Time,
}

impl FrontierModel for TimeLimitFrontierModel {
    fn valid_frontier(
        &self,
        _edge: &Edge,
        _previous_edge: Option<&Edge>,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        let time = state_model
            .get_time(state, bambam_feature::TRIP_TIME)
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let is_valid = time <= self.time_limit;
        Ok(is_valid)
    }

    fn valid_edge(&self, _edge: &Edge) -> Result<bool, FrontierModelError> {
        Ok(true)
    }
}
