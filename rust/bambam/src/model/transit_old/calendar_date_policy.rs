use std::collections::HashSet;

use chrono::{DateTime, Utc};
use gtfs_structures::Gtfs;
use serde::{Deserialize, Serialize};

use super::schedule_error::ScheduleError;

/// a calendar policy describes how to select valid trips from a scheduled dataset.
///
/// GTFS data, for example, lists trips on routes by their service_id, which references
/// the date range and days of the week where a trip is valid. it is important to select
/// _some_ date, since there may be any number of trips of the same route
/// for matching datetime ranges in a GTFS archive. but in a deterministic search context,
/// we should have exactly one of each of these trips.
///  
/// the different policies provide different semantics for dealing with schedules:
///   1. a [Date] policy targets one single day without any fallback
///   2. a [DateDayOfWeek] policy will fill in missing route trips by testing matching
///      days of the week until it finds the nearest match
///   3. a [DateTimeRange] policy will fill in missing route trips by testing other days
///      in some datetime range
///
/// the selection of a policy depends on user preferences. should the application fail if
/// a date is selected that is not valid for some agency? or should
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename = "snake_case")]
pub enum CalendarDatePolicy {
    /// target a single day of a scheduled dataset.
    ///
    /// # Arguments
    ///
    /// * `date` - day to target for scheduled trips
    /// * `require_nonempty_agencies` - application will fail to load if any agency is empty
    ///                                 after applying this calendar policy
    Date {
        date: DateTime<Utc>,
        require_nonempty_agencies: bool,
    },

    /// target a single day of a scheduled dataset. optionally expand by closest
    /// matching day of week.
    ///
    /// # Arguments
    ///
    /// * `date` - day to target for scheduled trips
    /// * `require_nonempty_agencies` - application will fail to load if any agency is empty
    ///                                 after applying this calendar policy
    /// * `use_closest_matching_weekday` - if no trips are found in the requested time range, then expand
    ///                         the search to the nearest dates that match the same days of the
    ///                         week
    DateDayOfWeek {
        date: DateTime<Utc>,
        require_nonempty_agencies: bool,
        use_closest_matching_weekday: bool,
    },

    /// target a single day of a scheduled dataset. if trip does not run on the provided day,
    /// then move to other days on the provided day range.
    ///
    /// # Arguments
    ///
    /// * `start_inclusive` - exclude scheduled trip data before this date and time
    /// * `end_exclusive` - exclude scheduled trip data at and after this date and time
    /// * `require_nonempty_agencies` - application will fail to load if any agency is empty
    ///                                 after applying this calendar policy
    DateTimeRange {
        date: DateTime<Utc>,
        fallback_start_inclusive: DateTime<Utc>,
        fallback_end_exclusive: DateTime<Utc>,
        require_nonempty_agencies: bool,
        use_closest_times: bool,
    },
}

impl CalendarDatePolicy {
    /// uses the calendar policy to pick the valid service ids from a GTFS archive.
    ///
    pub fn get_gtfs_service_ids(
        &self,
        _agency_id: &str,
        _gtfs: &Gtfs,
    ) -> Result<HashSet<String>, ScheduleError> {
        todo!()
    }
}
