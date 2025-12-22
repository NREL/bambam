use std::path::Path;

use chrono::TimeDelta;

/// downloads GBFS data for some duration. aggregates the resulting rows and writes them
/// to files to be consumed by BAMBAM.
///
/// # Arguments
/// * url - URL to the GBFS dataset
/// * out_dir - output directory to write the processed GBFS data
/// * dur - how long to poll the GBFS API
///
/// # Result
/// If successful, returns nothing, otherwise an error
pub fn run_gbfs_download(url: &str, out_dir: &Path, dur: &TimeDelta) -> Result<(), String> {
    let dur_secs = dur.as_seconds_f64();
    log::debug!(
        "run_gbfs_download with url={url}, out_dir={out_dir:?}, duration (seconds)={dur_secs}"
    );
    todo!("download + post-processing logic")
}
