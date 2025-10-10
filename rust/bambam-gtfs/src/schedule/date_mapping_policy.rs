use std::sync::Arc;

use chrono::{Datelike, Days, NaiveDate};
use gtfs_structures::{Calendar, CalendarDate, Exception, Gtfs};
use serde::{Deserialize, Serialize};

use crate::schedule::{date_ops, schedule_error::ScheduleError, ProcessedTrip};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DateMappingPolicy {
    ExactDay(NaiveDate),
    ExactRange {
        /// start date in range
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
    MatchNearest {
        start_date: NaiveDate,
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
            DateMappingPolicy::ExactDay(day) => DateIterator {
                current: Some(*day),
                start_inclusive: *day,
                end_inclusive: *day,
            },
            DateMappingPolicy::ExactRange {
                start_date,
                end_date,
            } => DateIterator {
                current: Some(*start_date),
                start_inclusive: *start_date,
                end_inclusive: *end_date,
            },
            DateMappingPolicy::MatchNearest {
                start_date,
                end_date,
                ..
            } => DateIterator {
                current: Some(*start_date),
                start_inclusive: *start_date,
                end_inclusive: end_date.unwrap_or_else(|| *start_date),
            },
        }
    }

    pub fn pick_date(
        &self,
        target: &NaiveDate,
        trip: &ProcessedTrip,
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
    start_inclusive: NaiveDate,
    end_inclusive: NaiveDate,
}

impl Iterator for DateIterator {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        let next_current =
            date_ops::increment_date(&current, &self.start_inclusive, &self.end_inclusive).ok();
        self.current = next_current;
        next_current
    }
}

/// confirm the target to exist as a valid date for this trip in the GTFS dataset.
/// returns the target date if successful.
fn pick_exact_date(
    target: &NaiveDate,
    trip: &ProcessedTrip,
    gtfs: &Gtfs,
) -> Result<NaiveDate, ScheduleError> {
    let c_opt = gtfs.get_calendar(&trip.service_id).ok();
    let cd_opt = gtfs.get_calendar_date(&trip.service_id).ok();
    match (c_opt, cd_opt) {
        (None, None) => {
            let msg = format!("cannot pick date with trip_id '{}' as it does not match calendar or calendar dates", trip.trip_id);
            Err(ScheduleError::MalformedGtfsError(msg))
        }
        (Some(c), None) => find_in_calendar(target, c),
        (None, Some(cd)) => confirm_add(target, cd),
        (Some(c), Some(cd)) => match find_in_calendar(target, c) {
            Ok(_) => confirm_no_delete(target, cd),
            Err(ce) => confirm_add(target, cd)
                .map_err(|e| ScheduleError::InvalidDataError(format!("{ce}, {e}"))),
        },
    }
}

fn pick_nearest_date(
    target: &NaiveDate,
    trip: &ProcessedTrip,
    gtfs: &Gtfs,
    date_tolerance: u64,
    match_weekday: bool,
) -> Result<NaiveDate, ScheduleError> {
    let c_opt = gtfs.get_calendar(&trip.service_id).ok();
    let cd_opt = gtfs.get_calendar_date(&trip.service_id).ok();

    todo!()
}

/// helper function to find some expected date in the calendar.txt of a GTFS archive
fn find_in_calendar(target: &NaiveDate, calendar: &Calendar) -> Result<NaiveDate, ScheduleError> {
    let start = &calendar.start_date;
    let end = &calendar.end_date;
    let within_service_date_range = start <= target && target <= end;
    if within_service_date_range {
        Ok(*target)
    } else {
        let msg = range_match_error_msg(target, start, end);
        Err(ScheduleError::InvalidDataError(format!(
            "no calendar.txt dates match target: {msg}"
        )))
    }
}

/// helper function to find some expected target date in the calendar_dates.txt of a
/// GTFS archive where the entry should have an exception_type of "Added".
fn confirm_add(
    target: &NaiveDate,
    calendar_dates: &[CalendarDate],
) -> Result<NaiveDate, ScheduleError> {
    match calendar_dates
        .iter()
        .find(|cd| &cd.date == target && cd.exception_type == Exception::Added)
    {
        Some(_) => Ok(*target),
        None => {
            let msg = format!(
                "no calendar_dates match target date '{}' with exception_type as 'added'",
                target.format("%m-%d-%Y"),
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
    }
}

/// helper function to find some expected target date in the calendar_dates.txt of a
/// GTFS archive where the entry should 1) not exist or 2) NOT have an exception_type of "Deleted".
fn confirm_no_delete(
    target: &NaiveDate,
    calendar_dates: &[CalendarDate],
) -> Result<NaiveDate, ScheduleError> {
    match calendar_dates
        .iter()
        .find(|cd| &cd.date == target && cd.exception_type == Exception::Deleted)
    {
        Some(_) => {
            let msg = format!(
                "date in calendar_dates match target date '{}' with exception_type as 'deleted'",
                target.format("%m-%d-%Y"),
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
        None => Ok(*target),
    }
}

/// helper for testing a date range with the range extended by some date tolerance
fn find_in_range_with_tolerance(
    target: &NaiveDate,
    start: &NaiveDate,
    end: &NaiveDate,
    tol: u64,
    match_weekday: bool,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let days = Days::new(tol);
    let start_with_tol = start.checked_sub_days(days).ok_or_else(|| {
        let start_str = start.format("%m-%d-%Y");
        let msg = format!("while applying date tolerance of {tol} days to start date {start_str}, date became out of range");
        ScheduleError::InvalidDataError(msg)
    })?;
    let end_with_tol = end.checked_add_days(days).ok_or_else(|| {
        let end_str = end.format("%m-%d-%Y");
        let msg = format!("while applying date tolerance of {tol} days to end date {end_str}, date became out of range");
        ScheduleError::InvalidDataError(msg)
    })?;

    // case 1) value is below start but within tolerance: [(start-tol) ... (target) ... (start)]
    let add_to_target = target < start && &start_with_tol <= target;
    // case 2) value is above end but within tolerance:  [(end) ... (target) ... (end+tol)]
    let sub_from_target = end < target && target <= &end_with_tol;

    if add_to_target && match_weekday {
        map_into_range_preserving_weekday(target, start, end).map(Some)
    } else if add_to_target {
        Ok(Some(*start))
    } else if sub_from_target && match_weekday {
        map_into_range_preserving_weekday(target, start, end).map(Some)
    } else if sub_from_target {
        Ok(Some(*end))
    } else {
        Ok(None)
    }
}

fn map_into_range_preserving_weekday(
    target: &NaiveDate,
    start: &NaiveDate,
    end: &NaiveDate,
) -> Result<NaiveDate, ScheduleError> {
    let is_below_range = target < start;
    let is_above_range = end < target;
    if !is_above_range && !is_below_range {
        let msg = format!(
            "invalid arguments for map_into_range_preserving_weekday, target {} is already within range [{},{}]",
            target.format("%m-%d-%Y"),
            start.format("%m-%d-%Y"),
            end.format("%m-%d-%Y")
        );
        return Err(ScheduleError::InternalError(msg));
    }
    // search for the nearest weekday starting from the closest range boundary to the target
    let mut cursor = if is_below_range { *start } else { *end };
    let one_day = Days::new(1);
    let one_week = Days::new(7);
    let stop = if is_below_range {
        cursor.checked_add_days(one_week).ok_or_else(|| {
            let cur_str = cursor.format("%m-%d-%Y");
            let msg = format!("while creating search boundary for range-preserving date mapping, was unable to add 7 days to {cur_str}");
            ScheduleError::InvalidDataError(msg)
        })
    } else {
        cursor.checked_sub_days(one_week).ok_or_else(|| {
            let cur_str = cursor.format("%m-%d-%Y");
            let msg = format!("while creating search boundary for range-preserving date mapping, was unable to subtract 7 days to {cur_str}");
            ScheduleError::InvalidDataError(msg)
        })
    }?;
    while cursor != stop {
        if cursor.weekday() == target.weekday() {
            // found it! terminate early!
            return Ok(cursor);
        }
        let next = cursor.checked_add_days(one_day).ok_or_else(|| {
            let cur_str = cursor.format("%m-%d-%Y");
            let msg = format!("while creating search boundary for range-preserving date mapping, was unable to add 1 day to {cur_str}");
            ScheduleError::InvalidDataError(msg)
        })?;
        cursor = next;
    }

    // didn't find a weekday to map into in the 'real' date range.
    let msg = format!(
        "unable to find matching weekday {} for target {} within range [{},{}]",
        target.weekday(),
        target.format("%m-%d-%Y"),
        start.format("%m-%d-%Y"),
        end.format("%m-%d-%Y")
    );
    Err(ScheduleError::InvalidDataError(msg))
}

fn range_match_error_msg(current: &NaiveDate, start: &NaiveDate, end: &NaiveDate) -> String {
    format!(
        "target date '{}' does not match [{},{}]",
        current.format("%m-%d-%Y"),
        start.format("%m-%d-%Y"),
        end.format("%m-%d-%Y")
    )
}
