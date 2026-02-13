use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ZoneLookupConfig {
    /// collection of zone records
    pub zone_record_input_file: String,
    /// geometries for zones
    pub zone_geometry_input_file: String,
}
