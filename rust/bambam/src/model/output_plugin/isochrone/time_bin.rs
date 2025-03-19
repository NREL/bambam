use crate::model::bambam_state_ops;
use routee_compass_core::model::{
    state::{StateModel, StateModelError, StateVariable},
    unit::AsF64,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct TimeBin {
    pub min_time: u64,
    pub max_time: u64,
}

impl TimeBin {
    pub fn key(&self) -> String {
        format!("{}", self.max_time)
    }

    pub fn state_time_within_bin(
        &self,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<bool, StateModelError> {
        let time = bambam_state_ops::get_reachability_time_minutes(state, state_model)?;
        let time_u64 = time.as_f64() as u64;
        let within_bin = self.min_time <= time_u64 && time_u64 < self.max_time;
        Ok(within_bin)
    }
}
