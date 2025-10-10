use std::{collections::BinaryHeap, sync::Arc};

use chrono::{Datelike, Days, NaiveDate};
use clap::ValueEnum;
use gtfs_structures::{Calendar, CalendarDate, Exception, Gtfs};
use serde::{Deserialize, Serialize};

use crate::schedule::{date_ops, schedule_error::ScheduleError, ProcessedTrip};

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
        if current > self.end_inclusive {
            return None; // prevent unbounded iteration with faulty arguments
        }
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

/// for date policies that search for the nearest valid dates to the target date by a threshold
/// and optionally enforce matching weekday.
fn pick_nearest_date(
    target: &NaiveDate,
    trip: &ProcessedTrip,
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
        (None, Some(cd)) => find_nearest_add(target, cd, date_tolerance, match_weekday),
        (Some(c), None) => {
            let matches = find_in_range_with_tolerance(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            matches.first().cloned().ok_or_else(|| {
                ScheduleError::InternalError("empty find result should be unreachable".to_string())
            })
        }
        (Some(c), Some(cd)) => {
            let matches = find_in_range_with_tolerance(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            if let Some(date_match) = matches.iter().next() {
                let _ = confirm_no_delete(date_match, cd)?;
                return Ok(*date_match);
            }
            let calendar_dates_error =
                match find_nearest_add(target, cd, date_tolerance, match_weekday) {
                    Ok(nearest) => return Ok(nearest),
                    Err(e) => e,
                };

            let msg = if match_weekday && matches.is_empty() {
                format!(
                    "unable to find nearest for {} from calendar.txt and calendar_dates.txt. found no matches in calendar.txt matching weekday within tolerance of {} days and failed to find an add in calendar_dates.txt due to: {}",
                    target.format("%m-%d-%Y"),
                    date_tolerance,
                    calendar_dates_error
                )
            } else if match_weekday {
                format!(
                    "unable to find nearest for {} from calendar.txt and calendar_dates.txt. found {} matches in calendar.txt with matching weekday and within date tolerance of {} days, but were all 'deleted' exception_type entries in calendar_dates.txt. failed to find an add in calendar_dates.txt due to: {}",
                    target.format("%m-%d-%Y"),
                    matches.len(),
                    date_tolerance,
                    calendar_dates_error
                )
            } else {
                format!(
                    "unable to find nearest for {} from calendar.txt and calendar_dates.txt. found {} matches in calendar.txt within date tolerance of {} days, but were all 'deleted' exception_type entries in calendar_dates.txt. failed to find an add in calendar_dates.txt due to: {}",
                    target.format("%m-%d-%Y"),
                    matches.len(),
                    date_tolerance,
                    calendar_dates_error
                )
            };

            Err(ScheduleError::InternalError(msg))
        }
    }
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

// Create a wrapper type for BinaryHeap ordering
#[derive(Debug)]
struct DateCandidate(u64, CalendarDate);

impl Ord for DateCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .cmp(&other.0)
            .then_with(|| self.1.date.cmp(&other.1.date))
    }
}

impl PartialOrd for DateCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DateCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
            && self.1.date == other.1.date
            && self.1.exception_type == other.1.exception_type
            && self.1.service_id == other.1.service_id
    }
}

impl Eq for DateCandidate {}

/// finds the nearest date to the target date that has an exception_type of "Added"
/// which is within some date_tolerance.
fn find_nearest_add(
    target: &NaiveDate,
    calendar_dates: &[CalendarDate],
    date_tolerance: u64,
    match_weekday: bool,
) -> Result<NaiveDate, ScheduleError> {
    let mut heap = BinaryHeap::new();
    for date in calendar_dates.iter() {
        let matches_exception = date.exception_type == Exception::Added;
        let matches_weekday = if match_weekday {
            date.date.weekday() == target.weekday()
        } else {
            true
        };

        if matches_exception && matches_weekday {
            let time_delta = date.date.signed_duration_since(*target).abs();
            let days = time_delta.num_days() as u64;
            if days < date_tolerance {
                heap.push(DateCandidate(days, date.clone()));
            }
        }
    }
    match heap.pop() {
        Some(min_distance_date) => Ok(min_distance_date.1.date),
        None => {
            let mwd_str = if match_weekday {
                " with matching weekday"
            } else {
                ""
            };
            let msg = format!(
                "no Added entry in calendar_dates.txt within {date_tolerance} days of {}{}",
                target.format("%m-%d-%Y"),
                mwd_str
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
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
) -> Result<Vec<NaiveDate>, ScheduleError> {
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
        find_in_range_preserving_weekday(target, start, end)
    } else if add_to_target {
        find_in_range(target, start, tol, true)
    } else if sub_from_target && match_weekday {
        find_in_range_preserving_weekday(target, start, end)
    } else if sub_from_target {
        find_in_range(target, end, tol, false)
    } else {
        let msg = format!(
            "could not find date near {} within {} days from date range [{}, {}]",
            tol,
            target.format("%m-%d-%Y"),
            start.format("%m-%d-%Y"),
            end.format("%m-%d-%Y"),
        );
        Err(ScheduleError::InvalidDataError(msg))
    }
}

fn find_in_range(
    target: &NaiveDate,
    boundary: &NaiveDate,
    tol: u64,
    adding: bool,
) -> Result<Vec<NaiveDate>, ScheduleError> {
    let distance = boundary.signed_duration_since(*target).abs().num_days() as u64;
    let remaining = tol - distance;
    let mut output = vec![*boundary];
    for days in 0..remaining {
        let next_opt = if adding {
            boundary.checked_add_days(Days::new(days))
        } else {
            boundary.checked_sub_days(Days::new(days))
        };
        let next = next_opt.ok_or_else(|| {
            let target_str = target.format("%m-%d-%Y");
            let op = if adding {
                "adding"
            } else {
                "subtracting"
            };
            let msg = format!("while finding overlap in date range with tolerance of {tol} days to date {target_str}, date became out of range");
            ScheduleError::InvalidDataError(msg)
        })?;
        output.push(next);
    }
    Ok(output)
}

fn find_in_range_preserving_weekday(
    target: &NaiveDate,
    start: &NaiveDate,
    end: &NaiveDate,
) -> Result<Vec<NaiveDate>, ScheduleError> {
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
    let mut found = vec![];
    while cursor != stop {
        if cursor.weekday() == target.weekday() {
            // found it! terminate early!
            found.push(cursor);
        }
        let next = cursor.checked_add_days(one_day).ok_or_else(|| {
            let cur_str = cursor.format("%m-%d-%Y");
            let msg = format!("while creating search boundary for range-preserving date mapping, was unable to add 1 day to {cur_str}");
            ScheduleError::InvalidDataError(msg)
        })?;
        cursor = next;
    }

    if !found.is_empty() {
        Ok(found)
    } else {
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
}

fn range_match_error_msg(current: &NaiveDate, start: &NaiveDate, end: &NaiveDate) -> String {
    format!(
        "target date '{}' does not match [{},{}]",
        current.format("%m-%d-%Y"),
        start.format("%m-%d-%Y"),
        end.format("%m-%d-%Y")
    )
}
