use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::ModeConfiguration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModeBasedTimeModelConfiguration {
    pub modes: HashMap<String, ModeConfiguration>,
}
