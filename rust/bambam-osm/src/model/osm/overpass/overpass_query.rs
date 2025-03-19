use serde::{Deserialize, Serialize};

use super::FilterQuery;

/// top-level ADT for the Overpass API Language described at
/// <https://wiki.openstreetmap.org/wiki/Overpass_API/Language_Guide>
/// left here as an extension point for future work on this module.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OverpassQuery {
    OverpassFilterQuery { query: FilterQuery },
}
