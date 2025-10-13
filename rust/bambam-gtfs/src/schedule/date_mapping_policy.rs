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
            Ok(_) => {
                if confirm_no_delete_exception(target, cd) {
                    Ok(*target)
                } else {
                    Err(ScheduleError::InvalidDataError(format!(
                    "date {} is valid for calendar.txt but has exception of deleted in calendar_dates.txt",
                    target.format(APP_DATE_FORMAT)
                )))
                }
            }
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
                .filter(|date_match| confirm_no_delete_exception(date_match, cd))
                .collect_vec();

            matches_minus_delete.iter().min().cloned().ok_or_else(|| {
                ScheduleError::InvalidDataError(format!(
                    "no match found across calendar + calendar_dates {}",
                    error_msg_suffix(target, &c.start_date, &c.end_date)
                ))
            })
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
        // Reverse the ordering to make BinaryHeap work as min-heap for distance
        other
            .0
            .cmp(&self.0)
            .then_with(|| other.1.date.cmp(&self.1.date))
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
    !calendar_dates
        .iter()
        .any(|cd| &cd.date == target && cd.exception_type == Exception::Deleted)
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
            if days <= date_tolerance {
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

/// adds (or when step is negative, subtracts) days from a date.
fn step_date(date: NaiveDate, step: i64) -> Result<NaiveDate, ScheduleError> {
    if step == 0 {
        return Ok(date);
    }
    let stepped = if step < 0 {
        let step_days = Days::new(step.unsigned_abs());
        date.checked_sub_days(step_days)
    } else {
        let step_days = Days::new(step.unsigned_abs());
        date.checked_add_days(step_days)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_step_date_zero_step() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let result = step_date(date, 0).unwrap();
        assert_eq!(result, date);
    }

    #[test]
    fn test_step_date_positive_step() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 6, 20).unwrap();
        let result = step_date(date, 5).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_negative_step() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 6, 10).unwrap();
        let result = step_date(date, -5).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_cross_month_boundary_forward() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 28).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 7, 3).unwrap();
        let result = step_date(date, 5).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_cross_month_boundary_backward() {
        let date = NaiveDate::from_ymd_opt(2023, 7, 3).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 6, 28).unwrap();
        let result = step_date(date, -5).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_cross_year_boundary_forward() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 28).unwrap();
        let expected = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        let result = step_date(date, 5).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_cross_year_boundary_backward() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 12, 26).unwrap();
        let result = step_date(date, -10).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_leap_year() {
        let date = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(); // 2024 is a leap year
        let expected = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let result = step_date(date, 2).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_non_leap_year() {
        let date = NaiveDate::from_ymd_opt(2023, 2, 28).unwrap(); // 2023 is not a leap year
        let expected = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
        let result = step_date(date, 1).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_large_positive_step() {
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let result = step_date(date, 364).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_large_negative_step() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let expected = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let result = step_date(date, -364).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_step_date_overflow_positive() {
        // Test with a date close to the maximum representable date
        let date = NaiveDate::MAX;
        let result = step_date(date, 1);
        assert!(result.is_err());

        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("failure adding"));
            assert!(msg.contains("bounds error"));
        } else {
            panic!("Expected InvalidDataError for overflow");
        }
    }

    #[test]
    fn test_step_date_overflow_negative() {
        // Test with a date close to the minimum representable date
        let date = NaiveDate::MIN;
        let result = step_date(date, -1);
        assert!(result.is_err());

        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("failure subtracting"));
            assert!(msg.contains("bounds error"));
        } else {
            panic!("Expected InvalidDataError for underflow");
        }
    }

    #[test]
    fn test_step_date_error_message_format() {
        let date = NaiveDate::MAX;
        let result = step_date(date, 100);

        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("failure adding 100 days"));
            assert!(msg.contains(&date.format(APP_DATE_FORMAT).to_string()));
            assert!(msg.contains("bounds error"));
        } else {
            panic!("Expected InvalidDataError with specific message format");
        }
    }

    #[test]
    fn test_step_date_edge_case_one_day() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();

        // Test adding one day
        let result_add = step_date(date, 1).unwrap();
        let expected_add = NaiveDate::from_ymd_opt(2023, 6, 16).unwrap();
        assert_eq!(result_add, expected_add);

        // Test subtracting one day
        let result_sub = step_date(date, -1).unwrap();
        let expected_sub = NaiveDate::from_ymd_opt(2023, 6, 14).unwrap();
        assert_eq!(result_sub, expected_sub);
    }

    #[test]
    fn test_step_date_maximum_safe_values() {
        // Test with reasonably large values that should work
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();

        // Test large positive step
        let result_pos = step_date(date, 10000);
        assert!(result_pos.is_ok());

        // Test large negative step
        let result_neg = step_date(date, -10000);
        assert!(result_neg.is_ok());
    }

    // Helper function to create test CalendarDate
    fn create_calendar_date(date: NaiveDate, exception_type: Exception) -> CalendarDate {
        CalendarDate {
            service_id: "test_service".to_string(),
            date,
            exception_type,
        }
    }

    // Tests for confirm_add_exception
    #[test]
    fn test_confirm_add_exception_success() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(target, Exception::Added),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Deleted,
            ),
        ];

        let result = confirm_add_exception(&target, &calendar_dates);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), target);
    }

    #[test]
    fn test_confirm_add_exception_not_found() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Deleted,
            ),
        ];

        let result = confirm_add_exception(&target, &calendar_dates);
        assert!(result.is_err());
        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("no calendar_dates match target date"));
            assert!(msg.contains("06-15-2023")); // MM-DD-YYYY format
            assert!(msg.contains("exception_type as 'added'"));
        } else {
            panic!("Expected InvalidDataError");
        }
    }

    #[test]
    fn test_confirm_add_exception_deleted_entry_exists() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(target, Exception::Deleted), // Has the date but wrong exception type
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Added,
            ),
        ];

        let result = confirm_add_exception(&target, &calendar_dates);
        assert!(result.is_err());
    }

    #[test]
    fn test_confirm_add_exception_empty_calendar_dates() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![];

        let result = confirm_add_exception(&target, &calendar_dates);
        assert!(result.is_err());
    }

    // Tests for confirm_no_delete_exception
    #[test]
    fn test_confirm_no_delete_exception_no_delete() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(target, Exception::Added),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Added,
            ),
        ];

        let result = confirm_no_delete_exception(&target, &calendar_dates);
        assert!(result);
    }

    #[test]
    fn test_confirm_no_delete_exception_has_delete() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(target, Exception::Deleted),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Added,
            ),
        ];

        let result = confirm_no_delete_exception(&target, &calendar_dates);
        assert!(!result);
    }

    #[test]
    fn test_confirm_no_delete_exception_date_not_present() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Deleted,
            ),
        ];

        let result = confirm_no_delete_exception(&target, &calendar_dates);
        assert!(result); // True because target date is not in the list
    }

    #[test]
    fn test_confirm_no_delete_exception_empty_calendar_dates() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![];

        let result = confirm_no_delete_exception(&target, &calendar_dates);
        assert!(result); // True because no entries means no delete exceptions
    }

    // Tests for find_nearest_add_exception
    #[test]
    fn test_find_nearest_add_exception_exact_match() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ),
            create_calendar_date(target, Exception::Added),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Added,
            ),
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 0, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), target);
    }

    #[test]
    fn test_find_nearest_add_exception_nearest_within_tolerance() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(); // Friday
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 12).unwrap(),
                Exception::Added,
            ), // 3 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 18).unwrap(),
                Exception::Added,
            ), // 3 days after
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 20).unwrap(),
                Exception::Deleted,
            ), // Should be ignored
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_ok());
        // Should return the closer one (6/12, which is 3 days away)
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2023, 6, 12).unwrap()
        );
    }

    #[test]
    fn test_find_nearest_add_exception_with_weekday_matching() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(); // Thursday
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 12).unwrap(),
                Exception::Added,
            ), // Monday, 3 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 8).unwrap(),
                Exception::Added,
            ), // Thursday, 7 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 22).unwrap(),
                Exception::Added,
            ), // Thursday, 7 days after
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 10, true);
        assert!(result.is_ok());
        // Should return the closer Thursday (6/8, which is 7 days away but matches weekday)
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2023, 6, 8).unwrap()
        );
    }

    #[test]
    fn test_find_nearest_add_exception_outside_tolerance() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 1).unwrap(),
                Exception::Added,
            ), // 14 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 30).unwrap(),
                Exception::Added,
            ), // 15 days after
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_err());
        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("no Added entry in calendar_dates.txt"));
            assert!(msg.contains("within 5 days"));
            assert!(msg.contains("06-15-2023")); // MM-DD-YYYY format
        } else {
            panic!("Expected InvalidDataError");
        }
    }

    #[test]
    fn test_find_nearest_add_exception_no_weekday_match() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(); // Thursday
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 12).unwrap(),
                Exception::Added,
            ), // Monday
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 16).unwrap(),
                Exception::Added,
            ), // Friday
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 18).unwrap(),
                Exception::Added,
            ), // Sunday
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, true);
        assert!(result.is_err());
        if let Err(ScheduleError::InvalidDataError(msg)) = result {
            assert!(msg.contains("no Added entry in calendar_dates.txt"));
            assert!(msg.contains("with matching weekday"));
        } else {
            panic!("Expected InvalidDataError");
        }
    }

    #[test]
    fn test_find_nearest_add_exception_only_deleted_entries() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 14).unwrap(),
                Exception::Deleted,
            ),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                Exception::Deleted,
            ),
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 16).unwrap(),
                Exception::Deleted,
            ),
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_nearest_add_exception_empty_calendar_dates() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_nearest_add_exception_equal_distance_picks_earlier() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 12).unwrap(),
                Exception::Added,
            ), // 3 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 18).unwrap(),
                Exception::Added,
            ), // 3 days after
        ];

        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_ok());
        // With the reversed ordering, when distances are equal, it should pick the earlier date
        // because of the tie-breaker: other.1.date.cmp(&self.1.date)
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2023, 6, 12).unwrap()
        );
    }

    #[test]
    fn test_find_nearest_add_exception_tolerance_edge_case() {
        let target = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let calendar_dates = vec![
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(),
                Exception::Added,
            ), // Exactly 5 days before
            create_calendar_date(
                NaiveDate::from_ymd_opt(2023, 6, 9).unwrap(),
                Exception::Added,
            ), // 6 days before (outside tolerance)
        ];

        // With tolerance 4, the date that is exactly 4 days away should be excluded
        let result = find_nearest_add_exception(&target, &calendar_dates, 4, false);
        assert!(result.is_err()); // 5 days should be outside tolerance

        // But with tolerance 6, it should be included
        let result = find_nearest_add_exception(&target, &calendar_dates, 5, false);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2023, 6, 10).unwrap()
        );
    }
}
