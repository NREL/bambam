use std::{collections::BinaryHeap, sync::Arc};

use chrono::{Datelike, Days, NaiveDate};
use clap::ValueEnum;
use gtfs_structures::{Calendar, CalendarDate, Exception, Gtfs};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::schedule::{date_ops, schedule_error::ScheduleError, SortedTrip};
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
    start_inclusive: NaiveDate,
    end_inclusive: NaiveDate,
}

impl DateIterator {
    pub fn new(start: NaiveDate, end: Option<NaiveDate>) -> DateIterator {
        DateIterator {
            current: Some(start),
            start_inclusive: start,
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
        (Some(c), None) => find_in_calendar(target, c),
        (None, Some(cd)) => confirm_add_exception(target, cd),
        (Some(c), Some(cd)) => match find_in_calendar(target, c) {
            Ok(_) => if confirm_no_delete_exception(target, cd) {
                Ok(*target)
            } else {
                Err(ScheduleError::InvalidDataError(format!(
                    "date {} is valid for calendar.txt but has exception of deleted in calendar_dates.txt",
                    target.format(APP_DATE_FORMAT)
                )))
            },
            Err(ce) => confirm_add_exception(target, cd)
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
        (None, Some(cd)) => find_nearest_add_exception(target, cd, date_tolerance, match_weekday),
        (Some(c), None) => {
            let matches = date_range_intersection(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            matches.first().cloned().ok_or_else(|| {
                let msg = error_msg_suffix(target, &c.start_date, &c.end_date);
                ScheduleError::InvalidDataError(format!("could not find any matching dates {msg}"))
            })
        }
        (Some(c), Some(cd)) => {
            // find all matches across calendar.txt and calendar_dates.txt
            let mut matches = date_range_intersection(
                target,
                &c.start_date,
                &c.end_date,
                date_tolerance,
                match_weekday,
            )?;
            for date in cd.iter() {
                if date.date == *target && !(date.exception_type == Exception::Added) {
                    matches.push(date.date);
                }
            }
            let matches_minus_delete = matches
                .into_iter()
                .filter(|date_match| confirm_no_delete_exception(date_match, cd))
                .collect_vec();
            
            matches_minus_delete.iter().min().cloned()
                .ok_or_else(|| ScheduleError::InvalidDataError(format!(
                    "no match found across calendar + calendar_dates {}",
                    error_msg_suffix(target, &c.start_date, &c.end_date)
                )))

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

/// tests intersection of some target date with a date range.
/// returns all valid dates that could be used from this date range, filtering by the date
/// tolerance and weekday match criteria.
fn date_range_intersection(
    target: &NaiveDate,
    start: &NaiveDate,
    end: &NaiveDate,
    tol: u64,
    match_weekday: bool,
) -> Result<Vec<NaiveDate>, ScheduleError> {
    let mut candidates: Vec<(u64, NaiveDate)> = Vec::new();

    // Calculate the tolerance range around the target date
    let target_start = step_date(*target, -(tol as i64))?;
    let target_end = step_date(*target, tol as i64)?;

    // Find the overlap between [target_start, target_end] and [start, end]
    let overlap_start = std::cmp::max(target_start, *start);
    let overlap_end = std::cmp::min(target_end, *end);

    // If there's no overlap, return empty vector
    if overlap_start > overlap_end {
        return Ok(Vec::new());
    }

    // Iterate through the overlapping date range
    let mut current = overlap_start;
    while current <= overlap_end {
        let distance = current.signed_duration_since(*target).abs().num_days() as u64;

        // Check if this date is within tolerance
        if distance <= tol {
            // Check weekday matching if required
            if !match_weekday || current.weekday() == target.weekday() {
                candidates.push((distance, current));
            }
        }

        // Move to next day
        current = current.checked_add_days(Days::new(1)).ok_or_else(|| {
            let msg = "date iteration became out of range while processing date range intersection"
                .to_string();
            ScheduleError::InvalidDataError(msg)
        })?;
    }

    // Sort by distance from target (closest first)
    candidates.sort_by_key(|(distance, _)| *distance);

    // Extract just the dates from the (distance, date) tuples
    Ok(candidates.into_iter().map(|(_, date)| date).collect())
}

/// tests intersection of some simulated target date across the boundary of some
/// date range
fn date_range_intersection_for_exception_type(
    target: &NaiveDate,
    boundary: &NaiveDate,
    tol: u64,
    // adding: bool,
) -> Result<Vec<NaiveDate>, ScheduleError> {
    let adding: bool = target < boundary;
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
            let target_str = target.format(APP_DATE_FORMAT);
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

/// finds the dates with maching weekday to some target in a date range.
/// fails if no dates were found with matching weekday.
fn date_range_intersection_preserving_weekday(
    target: &NaiveDate,
    start: &NaiveDate,
    end: &NaiveDate,
) -> Result<Vec<NaiveDate>, ScheduleError> {
    let date_range = *start..*end;
    let first_weekday_opt =
        DateIterator::new(*start, Some(*end)).find(|d| d.weekday() == target.weekday());
    let first_weekday = match first_weekday_opt {
        Some(first) => Ok(first),
        None => {
            let msg = format!(
                "no date with matching weekday {}",
                error_msg_suffix(target, start, end)
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
    }?;

    // search through the rest of the range stepping by a week at a time
    let mut result = vec![];
    let mut cursor = Some(first_weekday);
    while let Some(prev_cursor) = cursor {
        let next_cursor = step_date(prev_cursor, 7)?;
        if date_range.contains(&next_cursor) {
            cursor = Some(next_cursor);
            result.push(next_cursor);
        } else {
            cursor = None;
        }
    }
    Ok(result)
}

/// helper function to find some expected target date in the calendar_dates.txt of a
/// GTFS archive where the entry should have an exception_type of "Added".
fn confirm_add_exception(
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
                target.format(APP_DATE_FORMAT),
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
    }
}

/// helper function to find some expected target date in the calendar_dates.txt of a
/// GTFS archive where the entry should 1) not exist or 2) NOT have an exception_type of "Deleted".
fn confirm_no_delete_exception(target: &NaiveDate, calendar_dates: &[CalendarDate]) -> bool {
    match calendar_dates
        .iter()
        .find(|cd| &cd.date == target && cd.exception_type == Exception::Deleted)
        .is_none()
}

/// finds the nearest date to the target date that has an exception_type of "Added"
/// which is within some date_tolerance.
fn find_nearest_add_exception(
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
                target.format(APP_DATE_FORMAT),
                mwd_str
            );
            Err(ScheduleError::InvalidDataError(msg))
        }
    }
}

fn range_match_error_msg(current: &NaiveDate, start: &NaiveDate, end: &NaiveDate) -> String {
    format!(
        "target date '{}' does not match [{},{}]",
        current.format(APP_DATE_FORMAT),
        start.format(APP_DATE_FORMAT),
        end.format(APP_DATE_FORMAT)
    )
}

fn step_date(date: NaiveDate, step: i64) -> Result<NaiveDate, ScheduleError> {
    if step == 0 {
        return Ok(date);
    }
    let stepped = if step < 0 {
        let step_days = Days::new(step.unsigned_abs());
        date.checked_add_days(step_days)
    } else {
        let step_days = Days::new(step.unsigned_abs());
        date.checked_sub_days(step_days)
    };
    stepped.ok_or_else(|| {
        let op = if step < 0 { "subtracting" } else { "adding" };
        let msg = format!(
            "failure {} {} days to date {} due to bounds error",
            op,
            step,
            date.format(APP_DATE_FORMAT)
        );
        ScheduleError::InvalidDataError(msg)
    })
}

/// helper function for returning errors that reference some target date and date range
fn error_msg_suffix(target: &NaiveDate, start: &NaiveDate, end: &NaiveDate) -> String {
    format!(
        "for target date '{}' and date range [{},{}]",
        target.format(APP_DATE_FORMAT),
        start.format(APP_DATE_FORMAT),
        end.format(APP_DATE_FORMAT)
    )
}
