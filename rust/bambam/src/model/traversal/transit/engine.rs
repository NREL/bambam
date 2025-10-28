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
        metadata::{self, GtfsArchiveMetadata},
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
        current_datetime: &NaiveDateTime,
    ) -> Result<(i64, Departure), TraversalModelError> {
        let departures_skiplists =
            self.edge_schedules
                .get(edge_id)
                .ok_or(TraversalModelError::InternalError(format!(
                    "EdgeId {edge_id} exceeds schedules length"
                )))?;

        // Collect next departure for each skiplist
        let infinity_datetime = Departure::infinity_from(*current_datetime).ok_or(
            TraversalModelError::InternalError(format!(
                "Failed to model infinity from {current_datetime}"
            )),
        )?;

        // Iterate over all routes that have schedules on this edge
        let result = departures_skiplists
            .iter()
            .map(|(route_id_label, skiplist)| {
                // reconcile with any date mappings. used to address date gaps across all GTFS archives.
                let search_datetime = self.apply_date_mapping(route_id_label, current_datetime);

                // Query the skiplist
                // We need to create the struct shell to be able to search the
                // skiplist. I tried several other approaches but I think this is the cleanest
                let search_query = Departure::construct_query(search_datetime);

                // get next or infinity. if infinity cannot be created: error
                let next_route_departure = skiplist
                    .lower_bound(std::ops::Bound::Included(&search_query))
                    .cloned()
                    .unwrap_or(Departure::infinity());

                // Return next departure for route
                (*route_id_label, next_route_departure)
            })
            .min_by_key(|(_, departure)| departure.dst_arrival_time)
            .ok_or(TraversalModelError::InternalError(
                "failed to find next departure: schedules for edge_id {edge_id} appear to be empty"
                    .to_string(),
            ))?;
        Ok(result)
    }

    /// checks for any date mapping for the current date/time value and applies it if found.
    pub fn apply_date_mapping(
        &self,
        route_id_label: &i64,
        current_datetime: &NaiveDateTime,
    ) -> NaiveDateTime {
        self.date_mapping
            .get(route_id_label)
            .and_then(|date_map| date_map.get(&current_datetime.date()))
            .unwrap_or(&current_datetime.date())
            .and_time(current_datetime.time())
    }
}

impl TryFrom<TransitTraversalConfig> for TransitTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(value: TransitTraversalConfig) -> Result<Self, Self::Error> {
        log::debug!(
            "loading transit traversal model from {}",
            value.gtfs_metadata_input_file
        );

        // Deserialize metadata file
        let file = File::open(value.gtfs_metadata_input_file).map_err(|e| {
            TraversalModelError::BuildError(format!("Failed to read metadata file: {e}"))
        })?;
        let metadata: GtfsArchiveMetadata =
            serde_json::from_reader(BufReader::new(file)).map_err(|e| {
                TraversalModelError::BuildError(format!("Failed to read metadata file: {e}"))
            })?;

        let route_id_to_state = match &value.route_ids_input_file {
            Some(route_ids_input_file) => MultimodalStateMapping::from_enumerated_category_file(
                Path::new(&route_ids_input_file),
            )?,
            None => MultimodalStateMapping::new(&metadata.fq_route_ids)?,
        };

        log::debug!(
            "loaded {} fq route ids into mapping",
            route_id_to_state.n_categories()
        );

        // re-map hash map keys from categorical to i64 label.
        let date_mapping = build_label_to_date_mapping(&metadata, &route_id_to_state)?;
        log::debug!("loaded date mapping with {} entries", date_mapping.len());

        let edge_schedules = read_schedules_from_file(
            value.edges_schedules_input_file,
            Arc::new(route_id_to_state),
            value.schedule_loading_policy,
        )?;

        Ok(Self {
            edge_schedules,
            date_mapping,
        })
    }
}

/// This function assumes that edge_id's are dense. If any edge_id is skipped, the transformation from
/// a HashMap into Vec<Schedule> will fail
fn read_schedules_from_file(
    filename: String,
    route_mapping: Arc<MultimodalStateMapping>,
    schedule_loading_policy: ScheduleLoadingPolicy,
) -> Result<Box<[HashMap<i64, Schedule>]>, TraversalModelError> {
    // Reading csv
    let rows: Box<[super::RawScheduleRow]> =
        read_utils::from_csv(&Path::new(&filename), true, None, None).map_err(|e| {
            TraversalModelError::BuildError(format!("Error creating reader to schedules file: {e}"))
        })?;

    log::debug!("{filename} - loaded {} raw schedule rows", rows.len());

    // Deserialize rows according to their edge_id
    let mut schedules: HashMap<usize, HashMap<i64, Schedule>> = HashMap::new();
    for record in rows {
        let route_i64 = route_mapping.get_label(&record.fully_qualified_id).ok_or(
            TraversalModelError::BuildError(format!(
                "Cannot find route id mapping for string {}",
                record.fully_qualified_id.clone()
            )),
        )?;

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
    log::debug!(
        "{filename} - built schedule lookup for {} routes",
        schedules.len()
    );

    // Observe total number of keys (edge_ids)
    let n_edges = schedules.keys().len();

    // Re-arrange all into a dense boxed slice
    let out = (0..n_edges)
        .map(|i| {
            schedules
                .remove(&i) // TIL: `remove` returns an owned value, consuming the hashmap
                .ok_or(TraversalModelError::BuildError(format!(
                    "Invalid schedules file. Missing edge_id {i} when the maximum edge_id is {n_edges}"
                )))
        })
        .collect::<Result<Vec<HashMap<i64, Schedule>>, TraversalModelError>>()?;

    log::debug!("{filename} - built skip lists for {} routes", out.len());

    Ok(out.into_boxed_slice())
}

/// helper function to construct a mapping from categorical label (a i64 StateVariable)
/// into a date mapping.
fn build_label_to_date_mapping(
    metadata: &GtfsArchiveMetadata,
    route_id_to_state: &MultimodalStateMapping,
) -> Result<HashMap<i64, HashMap<NaiveDate, NaiveDate>>, TraversalModelError> {
    let mapped = metadata
            .fq_route_ids
            .iter()
            .map(|route_id| {
                let label = route_id_to_state.get_label(route_id)
                    .ok_or_else(|| {
                        // this is only possible if the fq_route_ids are not the same as the dataset
                        // that created the state mapping.
                        TraversalModelError::BuildError(
                            "fully-qualified route id '{route_id}' has no entry in enumeration table from file".to_string()
                        )
                    })?;
                let mapping = match metadata.date_mapping.get(route_id) {
                    None => return Ok(None),
                    Some(mapping) => mapping,
                };
                Ok(Some((*label, mapping.clone())))
            })
            .collect::<Result<Vec<_>, TraversalModelError>>()?;
    let result = mapped.into_iter().flatten().collect();
    Ok(result)
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
        NaiveDateTime::parse_from_str(&format!("20250101 {string}"), "%Y%m%d %H:%M:%S").unwrap()
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
                    dummy_schedule(&[("16:00:00", "16:05:00"), ("16:25:00", "16:30:00")]),
                ),
                (1, dummy_schedule(&[("16:15:00", "16:45:00")])),
            ]),
            HashMap::from([
                (
                    0,
                    dummy_schedule(&[("16:05:00", "16:10:00"), ("16:30:00", "16:35:00")]),
                ),
                (1, dummy_schedule(&[("16:45:00", "17:00:00")])),
            ]),
        ];

        TransitTraversalEngine {
            edge_schedules: schedules.into_boxed_slice(),
            date_mapping: HashMap::new(),
        }
    }

    fn dummy_schedule(times: &[(&str, &str)]) -> Schedule {
        let departures = times.iter().map(|(src, dst)| Departure {
            src_departure_time: internal_date(src),
            dst_arrival_time: internal_date(dst),
        });
        Schedule::from_iter(departures)
    }

    #[test]
    fn test_get_next_departure() {
        let engine = get_dummy_engine();

        let mut current_edge: usize = 0;
        let mut current_time = internal_date("15:50:00");
        let mut next_tuple = engine
            .get_next_departure(current_edge, &current_time)
            .unwrap();
        let mut next_route = next_tuple.0;
        let mut next_departure = next_tuple.1;

        assert_eq!(next_route, 0);
        assert_eq!(next_departure.src_departure_time, internal_date("16:00:00"));

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
        assert_eq!(current_time, internal_date("16:45:00"));

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

    #[test]
    fn test_schedule_from_iter() {
        let departures = vec![
            Departure {
                src_departure_time: internal_date("10:00:00"),
                dst_arrival_time: internal_date("10:15:00"),
            },
            Departure {
                src_departure_time: internal_date("08:00:00"),
                dst_arrival_time: internal_date("08:20:00"),
            },
            Departure {
                src_departure_time: internal_date("09:00:00"),
                dst_arrival_time: internal_date("09:10:00"),
            },
        ];

        let schedule = Schedule::from_iter(departures);
        assert_eq!(schedule.len(), 3);

        // Should be ordered automatically
        let ordered: Vec<&Departure> = schedule.iter().collect();
        assert_eq!(ordered[0].src_departure_time, internal_date("08:00:00"));
        assert_eq!(ordered[1].src_departure_time, internal_date("09:00:00"));
        assert_eq!(ordered[2].src_departure_time, internal_date("10:00:00"));
    }

    #[test]
    fn test_departure_infinity() {
        let current_time = internal_date("15:30:00");
        let infinity_departure = Departure::infinity_from(current_time).unwrap();

        // Infinity should be 72 months (6 years) in the future
        let expected_infinity = current_time.checked_add_months(Months::new(72)).unwrap();
        assert_eq!(infinity_departure.src_departure_time, expected_infinity);
        assert_eq!(infinity_departure.dst_arrival_time, expected_infinity);

        // Any regular departure should be less than infinity
        let regular_departure = Departure {
            src_departure_time: internal_date("23:59:59"),
            dst_arrival_time: internal_date("23:59:59"),
        };
        assert!(regular_departure < infinity_departure);
    }

    #[test]
    fn test_schedule_comprehensive_search_scenario() {
        // Create a realistic bus schedule with multiple departures throughout the day
        let schedule = dummy_schedule(&[
            ("06:00:00", "06:25:00"), // Early morning
            ("06:30:00", "06:55:00"),
            ("07:00:00", "07:25:00"), // Rush hour starts
            ("07:15:00", "07:40:00"),
            ("07:30:00", "07:55:00"),
            ("08:00:00", "08:25:00"),
            ("09:00:00", "09:25:00"), // Off-peak
            ("10:00:00", "10:25:00"),
            ("17:00:00", "17:25:00"), // Evening rush
            ("17:30:00", "17:55:00"),
            ("18:00:00", "18:25:00"),
            ("22:00:00", "22:25:00"), // Late evening
        ]);

        // Test various search scenarios
        let test_cases = vec![
            ("05:30:00", Some("06:00:00")), // Before service starts
            ("06:00:00", Some("06:00:00")), // Exact match
            ("06:10:00", Some("06:30:00")), // Between departures
            ("07:20:00", Some("07:30:00")), // During rush hour
            ("12:00:00", Some("17:00:00")), // Large gap in service
            ("21:00:00", Some("22:00:00")), // Evening service
            ("23:00:00", None),             // After service ends
        ];

        for (search_time, expected_time) in test_cases {
            let search_departure = Departure {
                src_departure_time: internal_date(search_time),
                dst_arrival_time: internal_date(search_time),
            };

            let result = schedule.lower_bound(std::ops::Bound::Included(&search_departure));

            match expected_time {
                Some(expected) => {
                    assert!(
                        result.is_some(),
                        "Expected departure at {} for search time {}",
                        expected,
                        search_time
                    );
                    assert_eq!(
                        result.unwrap().src_departure_time,
                        internal_date(expected),
                        "Search time {} should find departure at {}",
                        search_time,
                        expected
                    );
                }
                None => {
                    assert!(
                        result.is_none(),
                        "Expected no departure for search time {}",
                        search_time
                    );
                }
            }
        }
    }
}
