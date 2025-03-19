use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(
    Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Deserialize, Serialize, Hash,
)]
pub struct OsmWayId(pub i64);

impl Display for OsmWayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
