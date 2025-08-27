use super::osm::OsmError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OsmCliError {
    #[error("failure reading run configuration: {0}")]
    ConfigurationError(String),
    #[error("failure reading .pbf file: {source}")]
    OsmError {
        #[from]
        source: OsmError,
    },
    #[error("failure reading configuration: {source}")]
    StdIoError {
        #[from]
        source: std::io::Error,
    },
    #[error("failure decoding JSON: {source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
}

// impl Display for OsmCliError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self)
//     }
// }
