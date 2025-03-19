use super::{
    calendar_date_policy::CalendarDatePolicy, schedule_type::ScheduleType,
    scheduled_edge::ScheduledEdge,
};

use routee_compass_core::model::network::{EdgeId, Graph, NetworkError};

pub struct ScheduledGraph {
    scheduled_edges: Vec<ScheduledEdge>,
    start_offset_id: EdgeId,
}

impl ScheduledGraph {
    /// read in from GTFS directory and collect all agencies, validating the
    /// archive and filtering the rows based on the calendar policy
    pub fn new(
        _schedule_type: &ScheduleType,
        _calendar_policy: &CalendarDatePolicy,
        _initial_graph: &Graph,
    ) -> Result<ScheduledGraph, NetworkError> {
        todo!()
    }

    /// extends the base graph so that it includes these scheduled graph edges.
    /// it then becomes the responsibility of the traversal model to manage
    /// how to use the old and new edges.
    pub fn append_scheduled_edges_to_graph(
        &self,
        _initial_graph: &Graph,
    ) -> Result<Graph, NetworkError> {
        todo!()
    }
}
