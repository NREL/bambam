use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use clap::ValueEnum;
use gtfs_structures::{Exception, Gtfs};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::schedule::date::date_ops;
use crate::schedule::{schedule_error::ScheduleError, SortedTrip};
use crate::util::date_codec::app::{
    deserialize_naive_date, deserialize_optional_naive_date, APP_DATE_FORMAT,
};

#[derive(Serialize, Deserialize, Clone, Debug, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DateMappingPolicyType {
    ExactDay,
    ExactRange,
    MatchNearest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DateMappingPolicy {
    #[serde(deserialize_with = "deserialize_naive_date")]
    ExactDay(NaiveDate),
    ExactRange {
        /// start date in range
        #[serde(deserialize_with = "deserialize_naive_date")]
        start_date: NaiveDate,
        #[serde(deserialize_with = "deserialize_naive_date")]
        end_date: NaiveDate,
    },
    MatchNearest {
        #[serde(deserialize_with = "deserialize_naive_date")]
        start_date: NaiveDate,
        #[serde(deserialize_with = "deserialize_optional_naive_date")]
        end_date: Option<NaiveDate>,
        /// limit to the number of days to search from the target date +-
        /// to a viable date in the GTFS archive.
        date_tolerance: u64,
        /// if true, choose the closest date that matches the same day of the
        /// week as our target date.
        match_weekday: bool,
    },
}

impl DateMappingPolicy {
    /// create an iterator over the dates we want to generate transit
    /// schedules for.
    pub fn iter(&self) -> DateIterator {
        match self {
            DateMappingPolicy::ExactDay(day) => DateIterator::new(*day, None),
            DateMappingPolicy::ExactRange {
                start_date,
                end_date,
            } => DateIterator::new(*start_date, Some(*end_date)),
            DateMappingPolicy::MatchNearest {
                start_date,
                end_date,
                ..
            } => DateIterator::new(*start_date, *end_date),
        }
    }

    pub fn pick_date(
        &self,
        target: &NaiveDate,
        trip: &SortedTrip,
        gtfs: Arc<Gtfs>,
    ) -> Result<NaiveDate, ScheduleError> {
        match self {
            DateMappingPolicy::ExactDay(_) => pick_exact_date(target, trip, &gtfs),
            DateMappingPolicy::ExactRange { .. } => pick_exact_date(target, trip, &gtfs),
            DateMappingPolicy::MatchNearest {
                date_tolerance,
                match_weekday,
                ..
            } => pick_nearest_date(target, trip, &gtfs, *date_tolerance, *match_weekday),
        }
    }
}

pub struct DateIterator {
    current: Option<NaiveDate>,
    end_inclusive: NaiveDate,
}

impl DateIterator {
    pub fn new(start: NaiveDate, end: Option<NaiveDate>) -> DateIterator {
        DateIterator {
            current: Some(start),
            end_inclusive: end.unwrap_or(start),
        }
    }
}

impl Iterator for DateIterator {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        if current > self.end_inclusive {
            return None; // prevent unbounded iteration with faulty arguments
        }
        let next_current = date_ops::step_date(current, 1).ok();
        self.current = next_current;
        next_current
    }
}

/// confirm the target to exist as a valid date for this trip in the GTFS dataset.
/// returns the target date if successful.
fn pick_exact_date(
    target: &NaiveDate,
    trip: &SortedTrip,
    gtfs: &Gtfs,
) -> Result<NaiveDate, ScheduleError> {
    let c_opt = gtfs.get_calendar(&trip.service_id).ok();
    let cd_opt = gtfs.get_calendar_date(&trip.service_id).ok();
    match (c_opt, cd_opt) {
        (None, None) => {
            let msg = format!("cannot pick date with trip_id '{}' as it does not match calendar or calendar dates", trip.trip_id);
            Err(ScheduleError::MalformedGtfsError(msg))
        }
        (Some(c), None) => date_ops::find_in_calendar(target, c),
        (None, Some(cd)) => date_ops::confirm_add_exception(target, cd),
        (Some(c), Some(cd)) => match date_ops::find_in_calendar(target, c) {
            Ok(_) => {
                if date_ops::confirm_no_delete_exception(target, cd) {
                    Ok(*target)
                } else {
                    Err(ScheduleError::InvalidDataError(format!(
                    "date {} is valid for calendar.txt but has exception of deleted in calendar_dates.txt",
                    target.format(APP_DATE_FORMAT)
                )))
                }
            }
            Err(ce) => date_ops::confirm_add_exception(target, cd)
                .map_err(|e| ScheduleError::InvalidDataError(format!("{ce}, {e}"))),
        },
    }
}

/// for date policies that search for the nearest valid dates to the target date by a threshold
/// and optionally enforce matching weekday.
fn pick_nearest_date(
    target: &NaiveDate,
    trip: &SortedTrip,
    gtfs: &Gtfs,
    date_tolerance: u64,
    match_weekday: bool,
) -> Result<NaiveDate, ScheduleError> {
    let c_opt = gtfs.get_calendar(&trip.service_id).ok();
    let cd_opt = gtfs.get_calendar_date(&trip.service_id).ok();
    match (c_opt, cd_opt) {
        (None, None) => {
            let msg = format!("cannot pick date with trip_id '{}' as it does not match calendar or calendar dates", trip.trip_id);
            Err(ScheduleError::MalformedGtfsError(msg))
        }
        (None, Some(cd)) => {
            date_ops::find_nearest_add_exception(target, cd, date_tolerance, match_weekday)
        }
        (Some(c), None) => {
            let matches = date_ops::date_range_intersection(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            matches.first().cloned().ok_or_else(|| {
                let msg = date_ops::error_msg_suffix(target, &c.start_date, &c.end_date);
                ScheduleError::InvalidDataError(format!("could not find any matching dates {msg}"))
            })
        }
        (Some(c), Some(cd)) => {
            // find all matches across calendar.txt and calendar_dates.txt
            let mut matches = date_ops::date_range_intersection(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            for calendar_date in cd.iter() {
                let matches_date = calendar_date.date == *target;
                let is_add = calendar_date.exception_type == Exception::Added;
                let matches_weekday_expectation =
                    !match_weekday || target.weekday() == calendar_date.date.weekday();
                if matches_date && is_add & matches_weekday_expectation {
                    matches.push(calendar_date.date);
                }
            }
            let matches_minus_delete = matches
                .into_iter()
                .filter(|date_match| date_ops::confirm_no_delete_exception(date_match, cd))
                .collect_vec();

            matches_minus_delete.iter().min().cloned().ok_or_else(|| {
                ScheduleError::InvalidDataError(format!(
                    "no match found across calendar + calendar_dates {}",
                    date_ops::error_msg_suffix(target, &c.start_date, &c.end_date)
                ))
            })
        }
    }
}
