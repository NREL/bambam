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
use chrono::{NaiveDate, NaiveDateTime};
use flate2::bufread::GzDecoder;
use routee_compass_core::{model::traversal::TraversalModelError, util::fs::read_utils};
use serde::{Deserialize, Serialize};
use skiplist::OrderedSkipList;
use uom::si::f64::Time;

pub struct TransitTraversalEngine {
    pub edge_schedules: Box<[HashMap<i64, Schedule>]>,
    pub date_mapping: HashMap<i64, HashMap<NaiveDate, NaiveDate>>,
}

impl TransitTraversalEngine {
    pub fn get_next_departure(
        &self,
        edge_id: usize,
        current_time: &NaiveDateTime,
    ) -> Result<(i64, Departure), TraversalModelError> {
        let departures_skiplists =
            self.edge_schedules
                .get(edge_id)
                .ok_or(TraversalModelError::InternalError(format!(
                    "EdgeId {edge_id} exceeds schedules length"
                )))?;

        // Collect next departure for each skiplist
        let infinity_datetime =
            Departure::infinity_from(*current_time).ok_or(TraversalModelError::InternalError(
                format!("Failed to model infinity from {current_time}"),
            ))?;

        // Iterate over all
        departures_skiplists
            .iter()
            .map(|(route_id, skiplist)| {
                // Map date
                let search_datetime = self
                    .date_mapping
                    .get(route_id)
                    .and_then(|date_map| date_map.get(&current_time.date()))
                    .unwrap_or(&current_time.date())
                    .and_time(current_time.time());

                // Query the skiplist
                // We need to create the struct shell to be able to search the
                // skiplist. I tried several other approaches but I think this is the cleanest
                let search_departure = Departure {
                    src_departure_time: search_datetime,
                    dst_arrival_time: search_datetime,
                };
                // get next or infinity. if infinity cannot be created: error
                let next_route_departure = skiplist
                    .lower_bound(std::ops::Bound::Included(&search_departure))
                    .unwrap_or(&infinity_datetime);

                // Return next departure for route
                (route_id, next_route_departure)
            })
            .min_by_key(|(_, &departure)| departure)
            .ok_or(TraversalModelError::InternalError("Failed to find minimum of vector of departures".to_string()))
            .map(|(&route, &departure)| (route, departure))
    }
}

impl TryFrom<TransitTraversalConfig> for TransitTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(value: TransitTraversalConfig) -> Result<Self, Self::Error> {
        // Deserialize metadata and extract route_ids
        let file = File::open(value.gtfs_metadata_input_file).map_err(|e| {
            TraversalModelError::BuildError(format!("Failed to read metadata file: {e}"))
        })?;
        let metadata: GtfsArchiveMetadata =
            serde_json::from_reader(BufReader::new(file)).map_err(|e| {
                TraversalModelError::BuildError(format!("Failed to read metadata file: {e}"))
            })?;

        let route_id_to_state = Arc::new(MultimodalStateMapping::new(&metadata.route_ids)?);

        // re-map hash map keys from categorical to i64 label
        let date_mapping = metadata
            .date_mapping
            .into_iter()
            .map(|(k, hash_map)| match route_id_to_state.get_label(&k) {
                Some(route_id) => Ok((*route_id, hash_map)),
                None => Err(TraversalModelError::BuildError(format!(
                    "failed to find label for categorical value: {k}"
                ))),
            })
            .collect::<Result<_, _>>()
            .map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failed to construct date mapping after matching route_id (str) to i64: {e}"
                ))
            })?;

        Ok(Self {
            edge_schedules: read_schedules_from_file(
                value.edges_schedules_input_file,
                route_id_to_state.clone(),
                value.schedule_loading_policy,
            )?,
            date_mapping,
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
) -> Result<Box<[HashMap<i64, Schedule>]>, TraversalModelError> {
    // Reading csv
    let rows: Box<[RawScheduleRow]> = read_utils::from_csv(&Path::new(&filename), true, None, None)
        .map_err(|e| {
            TraversalModelError::BuildError(format!("Error creating reader to schedules file: {e}"))
        })?;

    // Deserialize rows according to their edge_id
    let mut schedules: HashMap<usize, HashMap<i64, Schedule>> = HashMap::new();
    for record in rows {
        let route_i64 =
            route_mapping
                .get_label(&record.route_id)
                .ok_or(TraversalModelError::BuildError(format!(
                    "Cannot find route id mapping for string {}",
                    record.route_id.clone()
                )))?;

        // This step creates an empty skiplist for every edge we see, even if we don't load any departures to it
        let schedule_skiplist = schedules
            .entry(record.edge_id)
            .or_default()
            .entry(*route_i64)
            .or_default();
        schedule_loading_policy.insert_if_valid(
            schedule_skiplist,
            Departure {
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
        .collect::<Result<Vec<HashMap<i64, Schedule>>, TraversalModelError>>()?;

    Ok(out.into_boxed_slice())
}

#[cfg(test)]
mod test {

    use crate::model::traversal::transit::{
        engine::TransitTraversalEngine,
        schedule::{Departure, Schedule},
    };
    use chrono::{Months, NaiveDateTime};
    use std::collections::HashMap;
    use std::str::FromStr;

    fn internal_date(string: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("20250101{string}"), "%Y%m%d%H%M%S").unwrap()
    }

    fn get_dummy_engine() -> TransitTraversalEngine {
        // There are two edges that reverse each other and two routes that move across them
        // Route 1:
        // 16:00 - 16:05 (A-B) -> 16:05 - 16:10 (B-A) -> 16:10 - 16:25 dwell -> 16:25 - 16:30 (A-B) -> 16:30 - 16:35 (B-A)
        //
        // Route 2:
        // 16:15 - 16:45 (A-B) -> 16:45 - 17:00 (B-A)

        let schedules: Vec<HashMap<i64, Schedule>> = vec![
            HashMap::from([
                (
                    0,
                    Schedule::from_iter(
                        vec![
                            Departure {
                                src_departure_time: internal_date("160000"),
                                dst_arrival_time: internal_date("160500"),
                            },
                            Departure {
                                src_departure_time: internal_date("162500"),
                                dst_arrival_time: internal_date("163000"),
                            },
                        ]
                        .into_iter(),
                    ),
                ),
                (
                    1,
                    Schedule::from_iter(
                        vec![Departure {
                            src_departure_time: internal_date("161500"),
                            dst_arrival_time: internal_date("164500"),
                        }]
                        .into_iter(),
                    ),
                ),
            ]),
            HashMap::from([
                (
                    0,
                    Schedule::from_iter(
                        vec![
                            Departure {
                                src_departure_time: internal_date("160500"),
                                dst_arrival_time: internal_date("161000"),
                            },
                            Departure {
                                src_departure_time: internal_date("163000"),
                                dst_arrival_time: internal_date("163500"),
                            },
                        ]
                        .into_iter(),
                    ),
                ),
                (
                    1,
                    Schedule::from_iter(
                        vec![Departure {
                            src_departure_time: internal_date("164500"),
                            dst_arrival_time: internal_date("170000"),
                        }]
                        .into_iter(),
                    ),
                ),
            ]),
        ];

        TransitTraversalEngine {
            edge_schedules: schedules.into_boxed_slice(),
            date_mapping: HashMap::new(),
        }
    }

    #[test]
    fn test_get_next_departure() {
        let engine = get_dummy_engine();

        let mut current_edge: usize = 0;
        let mut current_time = internal_date("155000");
        let mut next_tuple = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        let mut next_route = next_tuple.0;
        let mut next_departure = next_tuple.1;

        assert_eq!(next_route, 0);
        assert_eq!(next_departure.src_departure_time, internal_date("160000"));

        // Traverse 3 times the next edge
        for i in 0..3 {
            next_tuple = engine
                .get_next_departure(current_edge, &current_time)
                .unwrap();
            next_route = next_tuple.0;
            next_departure = next_tuple.1;

            current_time = next_departure.dst_arrival_time;
            current_edge = 1 - current_edge;
        }

        // At 16:15, the next departure changes route
        // That is because route 0 is dwelling until 16:25
        assert_eq!(next_route, 1);
        assert_eq!(current_time, internal_date("164500"));

        // Ride transit one more time
        next_tuple = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        next_route = next_tuple.0;
        next_departure = next_tuple.1;

        current_time = next_departure.dst_arrival_time;
        current_edge = 1 - current_edge;

        // If we wait now, we will find there are no more departures
        next_tuple = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        next_route = next_tuple.0;
        next_departure = next_tuple.1;
        assert_eq!(
            next_departure.src_departure_time,
            Departure::infinity_from(current_time)
                .unwrap()
                .src_departure_time
        );
    }
}
