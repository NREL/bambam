use serde::{Deserialize, Serialize};
use super::{ RowFilterConfig, ReleaseVersion };

#[derive(Debug, Serialize, Deserialize)]
struct CollectionRunConfig{
    data_type: DatasetType,
    release_version: ReleaseVersion,
    row_filter_config: Option<RowFilterConfig>
}

#[derive(Debug, Serialize, Deserialize)]
enum DatasetType{
    Places,
    Buildings
}