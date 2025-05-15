use super::{ReleaseVersion, RowFilterConfig};
use serde::{Deserialize, Serialize};

/// Serializable configuration of a run. Once the collector object has
/// been initialized, this data allows the retrieval of a dataset.
#[derive(Debug, Serialize, Deserialize)]
struct CollectionRunConfig {
    data_type: DatasetType,
    release_version: ReleaseVersion,
    row_filter_config: Option<RowFilterConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
enum DatasetType {
    Places,
    Buildings,
}
