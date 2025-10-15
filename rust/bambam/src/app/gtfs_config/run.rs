use std::path::Path;

use config::Config;

use crate::app::gtfs_config::gtfs_config_error::GtfsConfigError;

pub fn run(directory: &Path, base_config_filepath: &str) -> Result<(), GtfsConfigError> {
    let base_config_file = config::File::new(base_config_filepath, config::FileFormat::Toml);
    let config = Config::builder()
        .add_source(base_config_file)
        .build()
        .map_err(|e| GtfsConfigError::ConfigReadError {
            msg: format!("failed reading '{base_config_filepath}'"),
            source: e,
        })?;
    let config_json = config
        .clone()
        .try_deserialize::<serde_json::Value>()
        .map_err(|e| GtfsConfigError::ConfigReadError {
            msg: format!("failed converting '{base_config_filepath}' to JSON"),
            source: e,
        })?;

    // TODO: for each unique set of (edges, schedules, metadata) in directory (or, create a manifest):
    //   1. step into [graph] to append the edge list file
    //   2. step into [search] to append traversal + frontier model configurations
    Ok(())
}
