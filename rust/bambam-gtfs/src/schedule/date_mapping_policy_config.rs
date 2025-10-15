use chrono::NaiveDate;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::schedule::{date::date_codec::app::APP_DATE_FORMAT, schedule_error::ScheduleError};

/// used to tag the type of mapping policy when constructing from CLI.
#[derive(Serialize, Deserialize, Clone, Debug, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DateMappingPolicyType {
    ExactDate,
    ExactRange,
    NearestDate,
    NearestRange,
}

/// configures a [`DateMappingPolicy`]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DateMappingPolicyConfig {
    ExactDate(String),
    ExactRange {
        /// start date in range
        start_date: String,
        end_date: String,
    },
    NearestDate {
        date: String,
        /// limit to the number of days to search from the target date +-
        /// to a viable date in the GTFS archive.
        date_tolerance: u64,
        /// if true, choose the closest date that matches the same day of the
        /// week as our target date.
        match_weekday: bool,
    },
    NearestRange {
        start_date: String,
        end_date: String,
        /// limit to the number of days to search from the target date +-
        /// to a viable date in the GTFS archive.
        date_tolerance: u64,
        /// if true, choose the closest date that matches the same day of the
        /// week as our target date.
        match_weekday: bool,
    },
}

impl DateMappingPolicyConfig {
    /// build a new [`DateMappingPolicy`] configuration from CLI arguments.
    pub fn new(
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        date_mapping_policy: &DateMappingPolicyType,
        date_mapping_date_tolerance: Option<u64>,
        date_mapping_match_weekday: Option<bool>,
    ) -> Result<DateMappingPolicyConfig, ScheduleError> {
        use DateMappingPolicyConfig as Config;
        use DateMappingPolicyType as Type;
        match date_mapping_policy {
            Type::ExactDate => Ok(Config::ExactDate(
                start_date.format(APP_DATE_FORMAT).to_string(),
            )),
            Type::ExactRange => Ok(Config::ExactRange {
                start_date: start_date.format(APP_DATE_FORMAT).to_string(),
                end_date: end_date.format(APP_DATE_FORMAT).to_string(),
            }),
            Type::NearestDate => {
                let match_weekday = date_mapping_match_weekday.ok_or_else(|| ScheduleError::GtfsAppError(String::from("for nearest-date mapping, must specify 'match_weekday' as 'true' or 'false'")))?;
                let date_tolerance = date_mapping_date_tolerance.ok_or_else(|| {
                    ScheduleError::GtfsAppError(String::from(
                        "for nearest-date mapping, must specify a date_tolerance in [0, inf)",
                    ))
                })?;
                Ok(Self::NearestDate {
                    date: start_date.format(APP_DATE_FORMAT).to_string(),
                    date_tolerance,
                    match_weekday,
                })
            }
            Type::NearestRange => {
                let match_weekday = date_mapping_match_weekday.ok_or_else(|| ScheduleError::GtfsAppError(String::from("for nearest-date mapping, must specify 'match_weekday' as 'true' or 'false'")))?;
                let date_tolerance = date_mapping_date_tolerance.ok_or_else(|| {
                    ScheduleError::GtfsAppError(String::from(
                        "for nearest-date mapping, must specify a date_tolerance in [0, inf)",
                    ))
                })?;
                Ok(Self::NearestRange {
                    start_date: start_date.format(APP_DATE_FORMAT).to_string(),
                    end_date: end_date.format(APP_DATE_FORMAT).to_string(),
                    date_tolerance,
                    match_weekday,
                })
            }
        }
    }
}
