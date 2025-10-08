use std::{cmp, collections::HashMap, path::PathBuf, sync::Arc};

use crate::model::traversal::transit::{
    schedule::{Departure, Schedule},
    transit_traversal_config::TransitTraversalConfig,
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
        self,
        edge_id: usize,
        current_time: &NaiveDateTime,
    ) -> Result<Departure, TraversalModelError> {
        let departures_skiplist =
            self.edge_schedules
                .get(edge_id)
                .ok_or(TraversalModelError::InternalError(format!(
                    "EdgeId {} exceeds schedules length",
                    edge_id
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
            .ok_or(TraversalModelError::InternalError(format!(
                "Failed to find departure in skiplist"
            )))?
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

fn read_schedules_from_file(
    filename: String,
    route_mapping: Arc<HashMap<String, i64>>,
) -> Result<Box<[Schedule]>, TraversalModelError> {
    // Identify groups of rows by edge_id
    // Apply mapping to route_id
    // Create Skiplist per edge

    // Reading csv
    let file_path = PathBuf::from(filename);
    let reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(file_path.as_path())
        .map_err(|e| {
            TraversalModelError::BuildError(format!(
                "Error creating reader to schedules file: {}",
                e
            ))
        })?;

    // Deserialize rows according to their edge_id
    let mut max_edge_id: usize = 0;
    let mut schedules: HashMap<usize, Vec<Departure>> = HashMap::new();
    for row in reader.into_deserialize::<RawScheduleRow>() {
        let record = row.map_err(|e| {
            TraversalModelError::BuildError(format!(
                "Failed to deserialize row from schedules file: {}",
                e
            ))
        })?;

        // Update the max value
        max_edge_id = cmp::max(max_edge_id, record.edge_id);

        let route_i64 =
            route_mapping
                .get(&record.route_id)
                .ok_or(TraversalModelError::BuildError(format!(
                    "Cannot find route id mapping for string {}",
                    record.route_id.clone()
                )))?;

        schedules
            .entry(record.edge_id)
            .or_default()
            .push(Departure {
                route_id: *route_i64,
                src_departure_time: record.src_departure_time,
                dst_arrival_time: record.dst_arrival_time,
            });
    }

    // Re-arrange all into a dense boxed slice
    let mut out = (0..max_edge_id)
        .map(|i| {
            schedules
                .get(&i)
                .ok_or(TraversalModelError::BuildError(format!(
                    "Invalid schedules file. Missing edge_id {} when the maximum edge_id is {}",
                    i, max_edge_id
                )))
                .map(|v| v.clone().into_iter().collect())
        })
        .collect::<Result<Vec<Schedule>, TraversalModelError>>()?;

    Ok(out.into_boxed_slice())
}
