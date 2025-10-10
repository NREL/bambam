use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use gtfs_structures::{Calendar, Exception, Gtfs, Trip};

use crate::schedule::schedule_error::ScheduleError;

/// uses calendar.txt and calendar_dates.txt to test if a given Trip runs within the
/// time range [start_date, end_date].
pub fn find_trip_start_date(
    trip: &Trip,
    gtfs: &Gtfs,
    dates: Option<&HashMap<String, HashMap<NaiveDate, Exception>>>,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let calendar = gtfs.get_calendar(&trip.service_id).ok();
    match (calendar, dates) {
        // archive contains both calendar.txt and calendar_dates.txt file, so we have to consider date exceptions
        (Some(c), Some(cd)) => {
            let in_calendar = (c.start_date <= *end_date) && (*start_date <= c.end_date);
            // only test calendar dates within dates supported by both calendar + user arguments
            let query_start = std::cmp::max(*start_date, c.start_date);
            let query_end = std::cmp::min(*end_date, c.end_date);
            search_calendar_dates(&query_start, &query_end, in_calendar, &trip.service_id, cd)
        }

        // archive only contains calendar.txt, so we are looking for an intersection of two date ranges
        (Some(c), None) => search_calendar(start_date, end_date, c),

        // archive only contains calendar_dates.txt, so we are looking for a single addition that matches our date range
        (None, Some(cd)) => {
            search_calendar_dates(start_date, end_date, false, &trip.service_id, cd)
        }

        (None, None) => {
            let msg = format!("trip_id '{}' with service_id '{}' has no entry in either calendar.txt or calendar_dates.txt", trip.id, trip.service_id);
            Err(ScheduleError::MalformedGtfsError(msg))
        }
    }
}

/// tests matching two date ranges for an intersection. if [start_date, end_date] and
/// [c.start_date, c.end_date] have an intersecting range, we search that range for the
/// first date where the service is available for that given day of the week.
pub fn search_calendar(
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    c: &Calendar,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let query_start = std::cmp::max(*start_date, c.start_date);
    let query_end = std::cmp::min(*end_date, c.end_date);
    if query_end < query_start {
        Ok(None)
    } else {
        let mut current_date = query_start;
        while current_date <= query_end {
            let matches_weekday = match current_date.weekday() {
                chrono::Weekday::Mon => c.monday,
                chrono::Weekday::Tue => c.tuesday,
                chrono::Weekday::Wed => c.wednesday,
                chrono::Weekday::Thu => c.thursday,
                chrono::Weekday::Fri => c.friday,
                chrono::Weekday::Sat => c.saturday,
                chrono::Weekday::Sun => c.sunday,
            };
            if matches_weekday {
                return Ok(Some(current_date));
            }
            current_date = increment_date(&current_date, &query_start, &query_end)?;
        }

        Ok(None)
    }
}

/// helper function to test existence of an Exception within a date range.
/// the behavior of what we do when we encounter exceptions depends on if
/// our date range [start_date, end_date] was found to match the service in
/// calendar.txt.
///
/// terminates early for any of these 3 cases:
///   - case 1: date range is NOT in calendar.txt, but we found one matching date addition
///   - case 2: date range IS in calendar.txt, and we found one date without an exception
///   - case 3: date range IS in calendar.txt, and we found one date with an addition
///     - this case could also be an Error, but we count it here as just a redundancy
pub fn search_calendar_dates(
    query_start: &NaiveDate,
    query_end: &NaiveDate,
    date_range_in_calendar: bool,
    service_id: &str,
    dates: &HashMap<String, HashMap<NaiveDate, Exception>>,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let mut current_date = *query_start;
    while &current_date <= query_end {
        // if date range not in calendar, we are looking for _one_ addition in range
        // if date range in calendar, we are looking for _one_ date not deleted

        let date_lookup_opt = dates.get(service_id);
        let exception_opt = match date_lookup_opt {
            Some(lookup) => lookup.get(&current_date),
            None => None,
        };
        match (date_range_in_calendar, exception_opt) {
            (false, Some(Exception::Added)) => return Ok(Some(current_date)), // case 1: found one addition, exit
            (true, None) => return Ok(Some(current_date)), // case 2: not deleted or added <=> not deleted, exit
            (true, Some(Exception::Added)) => return Ok(Some(current_date)), // case 3: redundancy/bad data, but exit
            _ => {}
        }

        current_date = increment_date(&current_date, query_start, query_end)?;
    }
    Ok(None)
}

/// helper function to increment a date value by 1 day within some time range.
pub fn increment_date(
    current_date: &NaiveDate,
    range_start: &NaiveDate,
    range_end: &NaiveDate,
) -> Result<NaiveDate, ScheduleError> {
    current_date.succ_opt().ok_or_else(|| {
        let msg = format!(
            "Date overflow in service coverage check. cursor: '{}', date range: [{},{}]",
            current_date.format("%m-%d-%Y"),
            range_start.format("%m-%d-%Y"),
            range_end.format("%m-%d-%Y"),
        );
        ScheduleError::MalformedGtfsError(msg)
    })
}
