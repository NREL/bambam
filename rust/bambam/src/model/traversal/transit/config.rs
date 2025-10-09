// Questions
// - Should the engine create the edges in compass? No
// - If we are already in the same route, should we make transit_boarding_time 0 but still the travel time = dst_arrival - current_time
// - If Schedules = Box<[Schedule]>, how do we access the correct schedule if I have an edge_id? edge_id is usize

use serde::{Deserialize, Serialize};

use crate::model::traversal::transit::schedule_loading_policy::ScheduleLoadingPolicy;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransitTraversalConfig {
    /// edges-schedules file path from gtfs preprocessing
    pub edges_schedules_filename: String,
    /// policy by which to prune departures when reading schedules
    pub schedule_loading_policy: ScheduleLoadingPolicy,
    /// all route ids available in multimdal search. this ordering will be used
    /// to generate an enumeration used in state modeling.
    pub available_route_ids: Vec<String>,
}
