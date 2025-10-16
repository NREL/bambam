#[derive(thiserror::Error, Debug)]
pub enum GtfsConfigError {
    #[error("failed reading '{filepath}': {error}")]
    ReadError { filepath: String, error: String },
    #[error("failed to run bambam gtfs configuration script: {0}")]
    RunError(String),
    #[error("{0}")]
    InternalError(String),
    #[error("{msg}: {source}")]
    ConfigReadError {
        msg: String,
        source: config::ConfigError,
    },
}
