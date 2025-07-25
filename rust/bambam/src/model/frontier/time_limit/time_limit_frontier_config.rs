use serde::{Deserialize, Serialize};

use crate::model::frontier::time_limit::TimeLimitConfig;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeLimitFrontierConfig {
    pub time_limit: TimeLimitConfig,
}
