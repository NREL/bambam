use std::{cmp, collections::HashMap, path::PathBuf, sync::Arc};

use crate::model::traversal::transit::{
    config::TransitTraversalConfig,
    schedule::{Departure, Schedule},
    schedule_loading_policy::{self, ScheduleLoadingPolicy},
};
use chrono::NaiveDateTime;
use routee_compass_core::model::traversal::TraversalModelError;
use serde::{Deserialize, Serialize};
use skiplist::OrderedSkipList;
use uom::si::f64::Time;

pub struct TransitTraversalEngine {
    pub edge_schedules: Box<[Schedule]>,
}

impl TransitTraversalEngine {
    // TODO: TEST
    pub fn get_next_departure(
        &self,
        edge_id: usize,
        current_time: &NaiveDateTime,
    ) -> Result<Departure, TraversalModelError> {
        let departures_skiplist =
            self.edge_schedules
                .get(edge_id)
                .ok_or(TraversalModelError::InternalError(format!(
                    "EdgeId {edge_id} exceeds schedules length"
                )))?;

        // We need to create the struct shell to be able to search the
        // skiplist. I tried several other approaches but I think this is the cleanest
        let search_departure = Departure {
            route_id: 0,
            src_departure_time: *current_time,
            dst_arrival_time: *current_time,
        };

        Ok(departures_skiplist
            .lower_bound(std::ops::Bound::Included(&search_departure))
            .ok_or(TraversalModelError::InternalError(
                "Failed to find departure in skiplist".to_string(),
            ))?
            .clone())
    }
}

impl TryFrom<TransitTraversalConfig> for TransitTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(value: TransitTraversalConfig) -> Result<Self, Self::Error> {
        // TODO: Replace with MultiModalStateMapping
        let route_mapping = Arc::new(HashMap::<String, i64>::new());

        Ok(Self {
            edge_schedules: read_schedules_from_file(
                value.edges_schedules_filename,
                route_mapping,
                value.schedule_loading_policy,
            )?,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawScheduleRow {
    edge_id: usize,
    pub route_id: String,
    pub src_departure_time: NaiveDateTime,
    pub dst_arrival_time: NaiveDateTime,
}

/// This function assumes that edge_id's are dense. If any edge_id is skipped, the transformation from
/// a HashMap into Vec<Schedule> will fail
fn read_schedules_from_file(
    filename: String,
    route_mapping: Arc<HashMap<String, i64>>,
    schedule_loading_policy: ScheduleLoadingPolicy,
) -> Result<Box<[Schedule]>, TraversalModelError> {
    // Reading csv
    let file_path = PathBuf::from(filename);
    let reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(file_path.as_path())
        .map_err(|e| {
            TraversalModelError::BuildError(format!("Error creating reader to schedules file: {e}"))
        })?;

    // Deserialize rows according to their edge_id
    let mut schedules: HashMap<usize, Schedule> = HashMap::new();
    for row in reader.into_deserialize::<RawScheduleRow>() {
        let record = row.map_err(|e| {
            TraversalModelError::BuildError(format!(
                "Failed to deserialize row from schedules file: {e}"
            ))
        })?;

        let route_i64 =
            route_mapping
                .get(&record.route_id)
                .ok_or(TraversalModelError::BuildError(format!(
                    "Cannot find route id mapping for string {}",
                    record.route_id.clone()
                )))?;

        // This step creates an empty skiplist for every edge we see, even if we don't load any departures to it
        let schedule_skiplist = schedules.entry(record.edge_id).or_default();
        schedule_loading_policy.insert_if_valid(
            schedule_skiplist,
            Departure {
                route_id: *route_i64,
                src_departure_time: record.src_departure_time,
                dst_arrival_time: record.dst_arrival_time,
            },
        );
    }

    // Observe total number of keys (edge_ids)
    let n_edges = schedules.keys().len();

    // Re-arrange all into a dense boxed slice
    let mut out = (0..n_edges)
        .map(|i| {
            schedules
                .remove(&i) // TIL: `remove` returns an owned value, consuming the hashmap
                .ok_or(TraversalModelError::BuildError(format!(
                    "Invalid schedules file. Missing edge_id {i} when the maximum edge_id is {n_edges}"
                )))
        })
        .collect::<Result<Vec<Schedule>, TraversalModelError>>()?;

    Ok(out.into_boxed_slice())
}
