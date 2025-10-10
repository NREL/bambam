use serde::{Deserialize, Serialize};

// We can add more to this as needed
#[derive(Serialize, Deserialize)]
pub struct GtfsArchiveMetadata {
    // List of unique route identifiers used in the schedules
    pub route_ids: Vec<String>,
}
