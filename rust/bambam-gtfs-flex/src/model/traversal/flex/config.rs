use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::model::traversal::flex::ZoneId;

/// the data backing this traversal model, which varies by service type.
/// for more information, see the README.md for this crate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GtfsFlexTraversalConfig {
    /// In this service type, trips are assigned a src_zone_id when they board.
    ServiceTypeOne {
        /// enumerated zone id file with empty rows for edges that have no zone.
        edge_zone_input_file: String,
    },
    /// In this service type, trips are assigned a src_zone_id and departure_time when they board.
    ServiceTypeTwo {
        /// csv file with schema matching [ServiceTypeTwoRow]
        zone_time_lookup_input_file: String,
    },
}

/// represents a row in the Service Type Two data source CSV file.
pub struct ServiceTypeTwoRow {
    /// source travel zone supported by this row
    src_zone_id: ZoneId,
    /// start of time range supported by zone
    start_time: NaiveDateTime,
    /// end of time range supported by zone
    end_time: NaiveDateTime,
    /// comma-delimited list of zone ids that can be destinations for this row
    dst_zone_ids: String,
}
