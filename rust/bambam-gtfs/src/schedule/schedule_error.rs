use itertools::Itertools;
use routee_compass_core::model::map::MapError;

#[derive(thiserror::Error, Debug)]
pub enum ScheduleError {
    #[error("Failed to parse gtfs bundle file into `Gtfs` struct: {0}")]
    BundleReadError(#[from] gtfs_structures::Error), // { source: gtfs_structures::Error },
    #[error("failure running bambam_gtfs: {0}")]
    GtfsAppError(String),
    #[error("Failed to match point with spatial index: {source}")]
    SpatialIndexMapError {
        #[from]
        source: MapError,
    },
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
    #[error("Cannot find service in calendar.txt with service_id: {0}")]
    InvalidCalendarError(String),
    #[error("Cannot find service in calendar_dates.txt with service_id: {0}")]
    InvalidCalendarDatesError(String),
    #[error("Invalid Edges and schedules keys")]
    InvalidResultKeysError,
    #[error("error due to dataset contents: {0}")]
    InvalidDataError(String),
    #[error("GTFS archive is malformed: {0}")]
    MalformedGtfsError(String),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("errors encountered during batch bundle processing: {0}")]
    BatchProcessingError(String),
}

pub fn batch_processing_error(errors: &[ScheduleError]) -> ScheduleError {
    let concatenated = errors.iter().map(|e| e.to_string()).join("\n  ");
    ScheduleError::BatchProcessingError(format!("[\n  {concatenated}\n]"))
}
