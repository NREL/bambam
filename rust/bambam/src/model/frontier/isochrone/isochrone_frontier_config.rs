use serde::{Deserialize, Serialize};

use crate::model::frontier::isochrone::TimeLimit;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IsochroneFrontierConfig {
    pub time_limit: TimeLimit,
}
