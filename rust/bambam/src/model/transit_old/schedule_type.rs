use std::collections::HashSet;

use super::{
    calendar_assignment_policy::CalendarAssignmentPolicy, schedule_error::ScheduleError,
    scheduled_edge::ScheduledEdge,
};
use gtfs_structures::{Gtfs, Stop, Trip};
use routee_compass_core::model::network::{Graph, VertexId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename = "snake_case", tag = "type")]
pub enum ScheduleType {
    /// General Transit Feed Specification for Schedules (GTFS-Schedule)
    /// # Arguments
    /// * `path` - path to a single GTFS archive or a directory containing GTFS archive(s)
    GTFS { path: String },
}

impl ScheduleType {
    pub fn build(
        &self,
        _calendar_assignment_policy: &CalendarAssignmentPolicy,
        initial_graph: &Graph,
    ) -> Result<Vec<ScheduledEdge>, ScheduleError> {
        match self {
            ScheduleType::GTFS { path } => {
                let _gtfs = Gtfs::new(path).map_err(ScheduleError::GtfsError)?;
                // let _tree = VertexRTree::from_directed_graph(initial_graph);

                // let stuff: Result<Vec<i32>, ScheduleError> = gtfs
                //     .agencies
                //     .iter()
                //     .map(|agency| {
                //         let agency_id = agency.id.clone().ok_or_else(|| {
                //             ScheduleError::ExpectedOptionalValueToExist(String::from(
                //                 "gtfs.agency.id",
                //             ))
                //         })?;
                //         let calendar_date_policy =
                //             calendar_assignment_policy.get_policy(&agency_id)?;
                //         let service_ids =
                //             calendar_date_policy.get_gtfs_service_ids(&agency_id, &gtfs)?;
                //         for (trip_id, trip) in gtfs.trips {
                //             if service_ids.contains(&trip.service_id) {
                //                 // trip takes place on our calendar dates
                //                 let route = gtfs.routes.get(&trip.route_id).ok_or_else(|| {
                //                     ScheduleError::GtfsMissingEntryForId(
                //                         String::from("route"),
                //                         trip.route_id,
                //                     )
                //                 })?;
                //             }
                //         }
                //         // todo:
                //         //   - step through routes, find trips for routes using the policy
                //         //   - to take pairs of stop times and make schedule edges out of them

                //         // dates
                //         // - use agency.timezone with calendar/calendar_dates date and stop_time time (second of day)
                //         //   to re-construct a valid DateTime<Utc>?
                //         // - calendar policy exposes methods for "valid date" that takes the agency.timezone + calendar date?
                //         //   - but no, it's more like we need a "find dates for trips" method or something, to invert the logic
                //         //   -

                //         Ok(1)
                //     })
                //     .collect::<Result<Vec<_>, _>>();
                todo!()
            }
        }
    }
}

pub fn trip_to_headways(
    _trip_id: &str,
    trip: &Trip,
    gtfs: &Gtfs,
    service_ids: &HashSet<String>,
) -> Result<Vec<()>, ScheduleError> {
    if service_ids.contains(&trip.service_id) {
        // trip takes place on our calendar dates
        let _route = gtfs.routes.get(&trip.route_id).ok_or_else(|| {
            ScheduleError::GtfsMissingEntryForId(String::from("route"), trip.route_id.clone())
        })?;
    }
    todo!()
}
