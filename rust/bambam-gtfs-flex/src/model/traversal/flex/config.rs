use serde::{Deserialize, Serialize};

/// the data backing this traversal model, which varies by service type.
/// for more information, see the README.md for this crate.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GtfsFlexTraversalConfig {
    /// archive file to read, a .zip GTFS Flex archive.
    /// for more information, see <https://gtfs.org/community/extensions/flex/>
    archive_input_file: String,
}
