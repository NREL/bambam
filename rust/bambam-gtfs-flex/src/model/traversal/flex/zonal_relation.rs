use chrono::NaiveDateTime;
use std::collections::BTreeMap;

use crate::model::traversal::flex::ZoneId;

/// the zone on the other end of a relationship to some source zone.
/// this may be unscheduled, which means it is always true, or it may
/// occur on some schedule, a partial function over time.
pub enum ZonalRelation {
    UnscheduledRelation(ZoneId),
    ScheduledRelation {
        dst_zone_id: ZoneId,
        schedule: ScheduledRelationsToZone,
    },
}

/// a schedule contains (possibly non-overlapping) time intervals for zone access.
/// uses BTreeMap for efficient O(log n) point-in-time queries.
#[derive(Debug, Clone)]
pub struct ScheduledRelationsToZone {
    // Key: start_time, Value: full schedule entry
    intervals: BTreeMap<NaiveDateTime, ZoneAccessSchedule>,
}

impl ScheduledRelationsToZone {
    pub fn new() -> Self {
        Self {
            intervals: BTreeMap::new(),
        }
    }

    /// Insert a new schedule interval. Returns an error if it overlaps with existing intervals.
    pub fn insert(&mut self, schedule: ZoneAccessSchedule) -> Result<(), String> {
        // Check for overlap with previous interval
        if let Some((_, prev)) = self.intervals.range(..schedule.start_time).next_back() {
            if prev.end_time > schedule.start_time {
                return Err(format!(
                    "Overlap detected: previous interval ends at {:?}, new starts at {:?}",
                    prev.end_time, schedule.start_time
                ));
            }
        }

        // Check for overlap with next interval
        if let Some((_, next)) = self.intervals.range(schedule.start_time..).nth(0) {
            if schedule.end_time > next.start_time {
                return Err(format!(
                    "Overlap detected: new interval ends at {:?}, next starts at {:?}",
                    schedule.end_time, next.start_time
                ));
            }
        }

        self.intervals.insert(schedule.start_time, schedule);
        Ok(())
    }

    /// Find the interval containing the given time, if any.
    /// Returns None if the time falls in a gap or outside all intervals.
    pub fn find_containing_interval(&self, time: NaiveDateTime) -> Option<&ZoneAccessSchedule> {
        // Find the largest start_time <= time
        self.intervals
            .range(..=time)
            .next_back()
            .and_then(|(_, schedule)| {
                // Check if time is within [start_time, end_time)
                if time < schedule.end_time {
                    Some(schedule)
                } else {
                    None
                }
            })
    }

    /// Returns true if there are no scheduled intervals.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Returns the number of scheduled intervals.
    pub fn len(&self) -> usize {
        self.intervals.len()
    }
}

/// represents a time interval during which a zone is accessible.
#[derive(Debug, Clone, Eq)]
pub struct ZoneAccessSchedule {
    /// start time of the access window (inclusive)
    pub start_time: NaiveDateTime,
    /// end time of the access window (exclusive)
    pub end_time: NaiveDateTime,
}

impl ZoneAccessSchedule {
    pub fn new(start_time: NaiveDateTime, end_time: NaiveDateTime) -> Self {
        Self {
            start_time,
            end_time,
        }
    }

    /// Returns true if this interval contains the given time.
    pub fn contains(&self, time: NaiveDateTime) -> bool {
        self.start_time <= time && time < self.end_time
    }

    /// Returns true if this interval overlaps with another.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start_time < other.end_time && other.start_time < self.end_time
    }
}

impl PartialEq for ZoneAccessSchedule {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time && self.end_time == other.end_time
    }
}

impl PartialOrd for ZoneAccessSchedule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ZoneAccessSchedule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Lexicographic ordering: first by start_time, then by end_time
        self.start_time
            .cmp(&other.start_time)
            .then_with(|| self.end_time.cmp(&other.end_time))
    }
}
