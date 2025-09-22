use routee_compass_core::model::{
    frontier::FrontierModelError,
    state::{StateModel, StateVariable},
    unit::TimeUnit,
};
use std::collections::{HashMap, HashSet};
use uom::si::f64::Time;

pub enum MultimodalConstraint {
    AllowedModes(HashSet<String>),
    ModeCounts(HashMap<String, usize>),
    MaxTripLegs(usize),
    MaxTime(HashMap<String, Time>),
}

impl MultimodalConstraint {
    pub fn valid_frontier(
        &self,
        edge_mode: &str,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<bool, FrontierModelError> {
        match self {
            MultimodalConstraint::AllowedModes(items) => todo!(),
            MultimodalConstraint::ModeCounts(hash_map) => todo!(),
            MultimodalConstraint::MaxTripLegs(n_legs) => {
                todo!("if we add this edge, do we exceed our max legs?")
            }
            MultimodalConstraint::MaxTime(hash_map) => todo!(),
        }
    }
}
