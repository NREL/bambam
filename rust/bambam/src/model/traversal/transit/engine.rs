use std::{
    cmp,
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::model::{
    state::{MultimodalMapping, MultimodalStateMapping},
    traversal::transit::{
        config::TransitTraversalConfig,
        metadata::GtfsArchiveMetadata,
        schedule::{Departure, Schedule},
        schedule_loading_policy::{self, ScheduleLoadingPolicy},
    },
};
use chrono::NaiveDateTime;
use flate2::bufread::GzDecoder;
use routee_compass_core::{model::traversal::TraversalModelError, util::fs::read_utils};
use serde::{Deserialize, Serialize};
use skiplist::OrderedSkipList;
use uom::si::f64::Time;

pub struct TransitTraversalEngine {
    pub edge_schedules: Box<[Schedule]>,
}

impl TransitTraversalEngine {
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
            .unwrap_or(&Departure::infinity_from(*current_time).ok_or(
                TraversalModelError::InternalError(format!(
                    "Failed to model infinity from {}",
                    current_time
                )),
            )?)
            .clone())
    }
}

impl TryFrom<TransitTraversalConfig> for TransitTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(value: TransitTraversalConfig) -> Result<Self, Self::Error> {
        // Deserialize metadata and extract route_ids
        let file = File::open(value.gtfs_metadata_input_file).map_err(|e| {
            TraversalModelError::BuildError(format!("Failed to read metadata file: {}", e))
        })?;
        let metadata: GtfsArchiveMetadata =
            serde_json::from_reader(BufReader::new(file)).map_err(|e| {
                TraversalModelError::BuildError(format!("Failed to read metadata file: {}", e))
            })?;

        let route_id_to_state = Arc::new(MultimodalMapping::new(&metadata.route_ids)?);
        Ok(Self {
            edge_schedules: read_schedules_from_file(
                value.edges_schedules_input_file,
                route_id_to_state.clone(),
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
    route_mapping: Arc<MultimodalStateMapping>,
    schedule_loading_policy: ScheduleLoadingPolicy,
) -> Result<Box<[Schedule]>, TraversalModelError> {
    // Reading csv
    let rows: Box<[RawScheduleRow]> = read_utils::from_csv(&Path::new(&filename), true, None, None)
        .map_err(|e| {
            TraversalModelError::BuildError(format!("Error creating reader to schedules file: {e}"))
        })?;

    // Deserialize rows according to their edge_id
    let mut schedules: HashMap<usize, Schedule> = HashMap::new();
    for record in rows {
        let route_i64 =
            route_mapping
                .get_label(&record.route_id)
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

#[cfg(test)]
mod test {

    use crate::model::traversal::transit::{
        engine::TransitTraversalEngine,
        schedule::{Departure, Schedule},
    };
    use chrono::{Months, NaiveDateTime};
    use std::str::FromStr;

    fn internal_date(string: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("20250101{}", string), "%Y%m%d%H%M%S").unwrap()
    }

    fn get_dummy_engine() -> TransitTraversalEngine {
        // There are two edges that reverse each other and two routes that move across them
        // Route 1:
        // 16:00 - 16:05 (A-B) -> 16:05 - 16:10 (B-A) -> 16:10 - 16:25 dwell -> 16:25 - 16:30 (A-B) -> 16:30 - 16:35 (B-A)
        //
        // Route 2:
        // 16:15 - 16:45 (A-B) -> 16:45 - 17:00 (B-A)

        let schedules: Vec<Schedule> = vec![
            Schedule::from_iter(
                vec![
                    Departure {
                        route_id: 0,
                        src_departure_time: internal_date("160000"),
                        dst_arrival_time: internal_date("160500"),
                    },
                    Departure {
                        route_id: 0,
                        src_departure_time: internal_date("162500"),
                        dst_arrival_time: internal_date("163000"),
                    },
                    Departure {
                        route_id: 1,
                        src_departure_time: internal_date("161500"),
                        dst_arrival_time: internal_date("164500"),
                    },
                ]
                .into_iter(),
            ),
            Schedule::from_iter(
                vec![
                    Departure {
                        route_id: 0,
                        src_departure_time: internal_date("160500"),
                        dst_arrival_time: internal_date("161000"),
                    },
                    Departure {
                        route_id: 0,
                        src_departure_time: internal_date("163000"),
                        dst_arrival_time: internal_date("163500"),
                    },
                    Departure {
                        route_id: 1,
                        src_departure_time: internal_date("164500"),
                        dst_arrival_time: internal_date("170000"),
                    },
                ]
                .into_iter(),
            ),
        ];

        TransitTraversalEngine {
            edge_schedules: schedules.into_boxed_slice(),
        }
    }

    #[test]
    fn test_get_next_departure() {
        let engine = get_dummy_engine();

        let mut current_edge: usize = 0;
        let mut current_time = internal_date("155000");
        let mut next_departure = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();

        assert_eq!(next_departure.route_id, 0);
        assert_eq!(next_departure.src_departure_time, internal_date("160000"));

        // Traverse 3 times the next edge
        for i in 0..3 {
            next_departure = engine
                .get_next_departure(current_edge, &current_time)
                .unwrap();
            current_time = next_departure.dst_arrival_time.clone();
            current_edge = 1 - current_edge;
        }

        // At 16:15, the next departure changes route
        // That is because route 0 is dwelling until 16:25
        assert_eq!(next_departure.route_id, 1);
        assert_eq!(current_time, internal_date("164500"));

        // Ride transit one more time
        next_departure = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        current_time = next_departure.dst_arrival_time.clone();
        current_edge = 1 - current_edge;

        // If we wait now, we will find there are no more departures
        next_departure = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        assert_eq!(
            next_departure.src_departure_time,
            Departure::infinity_from(current_time)
                .unwrap()
                .src_departure_time
        );
    }
}
