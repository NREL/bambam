use super::{
    schedule_error::ScheduleError, schedule_id::ScheduleId, scheduled_headway::ScheduledHeadway,
};
use crate::model::transit_old::schedule_ops;
use chrono::{DateTime, Utc};
use gtfs_structures::{Gtfs, StopTime, Trip};
use routee_compass_core::model::{network::VertexId, unit::TimeUnit};
use skiplist::OrderedSkipList;
use uom::si::f64::Time;

pub struct ScheduledEdge {
    // edge_id: EdgeId,
    schedule_id: ScheduleId,
    headways: OrderedSkipList<ScheduledHeadway>,
}

impl ScheduledEdge {
    /// creates a new edge from some pair of stop times, and their nearest associated VertexIds.
    ///
    /// # Arguments
    ///
    /// * `trip` - the trip that traverses this new edge
    /// * `prev_stop` - in a pair of stops along a trip, the previous stop
    /// * `next_stop` - in a pair of stops along a trip, the next stop
    /// * `prev_vertex` - closest vertex in the graph to the previous stop
    /// * `next_vertex` - closest vertex in the graph to the next stop
    /// * `gtfs` - GTFS data archive
    ///
    /// # Returns
    ///
    /// A new edge for trips along this route, or an error
    pub fn new(
        _trip: &Trip,
        _prev_stop: &StopTime,
        _next_stop: &StopTime,
        _prev_vertex: VertexId,
        _next_vertex: VertexId,
        _gtfs: &Gtfs,
    ) -> Result<ScheduledEdge, ScheduleError> {
        todo!()
    }

    /// appends a headway to this edge.
    ///
    /// # Arguments
    ///
    /// * `prev_stop` - in a pair of stops along a trip, the previous stop
    /// * `next_stop` - in a pair of stops along a trip, the next stop
    ///
    /// # Returns
    ///
    /// nothing if successful, or an error
    pub fn append(
        &mut self,
        _prev_stop: &StopTime,
        _next_stop: &StopTime,
    ) -> Result<(), ScheduleError> {
        todo!()
    }

    /// finds the next available headway along this edge which departs
    /// on or after the current time.
    ///
    /// # Arguments
    ///
    /// * `start_datetime` - starting time of the trip
    /// * `current_time` - time at this point in the search, including any delays
    ///                    due to accessing this transit option
    /// * `time_unit` - time unit used by the search state
    ///
    /// # Returns
    ///
    /// The next available headway, or None if there is no option
    pub fn next_headway(
        &self,
        start_datetime: DateTime<Utc>,
        current_time: &Time,
    ) -> Result<Option<&ScheduledHeadway>, ScheduleError> {
        let current_datetime = schedule_ops::add_delta(start_datetime, *current_time)?;
        let comparator = ScheduledHeadway::query(current_datetime);
        let headway = self
            .headways
            .lower_bound(std::ops::Bound::Included(&comparator));

        Ok(headway)
    }
}
