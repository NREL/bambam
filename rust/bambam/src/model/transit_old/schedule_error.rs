use chrono::{DateTime, Utc};
use routee_compass_core::model::unit::TimeUnit;

#[derive(thiserror::Error, Debug)]
pub enum ScheduleError {
    #[error("expected optional value for {0} to exist")]
    ExpectedOptionalValueToExist(String),
    #[error("unknown agency id {0}")]
    UnknownAgencyId(String),
    #[error("could not add {0} {1} to {2}")]
    AddTimeToDateTimeError(f64, TimeUnit, DateTime<Utc>),
    #[error(transparent)]
    GtfsError(#[from] gtfs_structures::Error),
    #[error("missing {0} id {1}")]
    GtfsMissingEntryForId(String, String),
}
