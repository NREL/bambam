use std::borrow::Cow;

use crate::model::bambam_state_ops;
use routee_compass_core::model::{
    state::{StateModel, StateModelError, StateVariable},
    unit::{AsF64, Convert, Time, TimeUnit},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TimeBin {
    pub min_time: u64,
    pub max_time: u64,
}

impl TimeBin {
    pub fn key(&self) -> String {
        format!("{}", self.max_time)
    }

    /// grab the time bin's lower bound as a Time value in a specified time unit
    pub fn min_time(&self, time_unit: &TimeUnit) -> Time {
        to_time_value(self.min_time, time_unit)
    }

    /// grab the time bin's upper bound as a Time value in a specified time unit
    pub fn max_time(&self, time_unit: &TimeUnit) -> Time {
        to_time_value(self.max_time, time_unit)
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

fn to_time_value(time_bin_value: u64, time_unit: &TimeUnit) -> Time {
    let mut time = Cow::Owned(Time::from(time_bin_value as f64));
    TimeUnit::Minutes.convert(&mut time, time_unit);
    time.into_owned()
}
