#[derive(thiserror::Error, Debug)]
pub enum GtfsConfigError {
    #[error("failed to run bambam gtfs configuration script: {0}")]
    RunError(String),
    #[error("{msg}: {source}")]
    ConfigReadError {
        msg: String,
        source: config::ConfigError,
    },
}
