use routee_compass_core::model::map::MapError;

#[derive(thiserror::Error, Debug)]
pub enum ScheduleError {
    #[error("Failed to parse gtfs bundle file into `Gtfs` struct: {0}")]
    BundleReadError(#[from] gtfs_structures::Error), // { source: gtfs_structures::Error },
    #[error("Failed to match point with spatial index: {source}")]
    SpatialIndexMapError { source: MapError },
    #[error("Spatial index matched an edge instead of a vertex")]
    SpatialIndexIncorrectMapError,
    #[error("Missing lon,lat data and parent_location for stop: {0}")]
    MissingStopLocationAndParentError(String),
    #[error("Missing both arrival and departure times: {0}")]
    MissingAllStopTimesError(String),
    #[error("At least one of the stops in edge is missing shape distance traveled: {0} or {1}")]
    MissingShapeDistanceTraveledError(String, String),
    #[error("Failed to create vertex index: {0}")]
    FailedToCreateVertexIndexError(String),
    #[error("Cannot find calendar ID: {0}")]
    InvalidCalendarError(String),
    #[error("Invalid Edges and schedules keys")]
    InvalidResultKeysError,
    #[error("{0}")]
    OtherError(String),
}
