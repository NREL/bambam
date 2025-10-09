use chrono::{Duration, NaiveDate, NaiveDateTime};
use csv::QuoteStyle;
use flate2::{write::GzEncoder, Compression};
use geo::Point;
use gtfs_structures::{Calendar, Exception, Gtfs, Stop, StopTime, Trip};
use itertools::Itertools;
use kdam::{Bar, BarBuilder, BarExt};
use rayon::prelude::*;
use routee_compass_core::model::{
    map::{NearestSearchResult, SpatialIndex},
    network::{Edge, EdgeConfig},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    fs::File,
    path::Path,
    sync::{Arc, Mutex},
};
use uom::si::f64::Length;
use wkt::ToWkt;

use crate::schedule::{
    batch_processing_error,
    distance_calculation_policy::{compute_haversine, DistanceCalculationPolicy},
    schedule_error::ScheduleError,
    MissingStopLocationPolicy,
};

#[derive(Serialize, Deserialize, Debug)]
struct ScheduleConfig {
    edge_id: usize,
    src_departure_time: NaiveDateTime,
    dst_arrival_time: NaiveDateTime,
    route_id: String,
}

/// multithreaded GTFS processing.
pub fn process_bundles(
    bundle_directory_path: &Path,
    start_edge_list_id: &usize,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    spatial_index: Arc<SpatialIndex>,
    missing_stop_location_policy: &MissingStopLocationPolicy,
    distance_calculation_policy: &DistanceCalculationPolicy,
    output_directory: &Path,
    overwrite: bool,
    parallelism: usize,
) -> Result<(), ScheduleError> {
    let archive_paths = bundle_directory_path
        .read_dir()
        .map_err(|e| ScheduleError::GtfsAppError(format!("failure reading directory: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ScheduleError::GtfsAppError(format!("failure reading directory: {e}")))?;
    let chunk_size = archive_paths.len() / parallelism;

    // a progress bar shared across threads
    let bar: Arc<Mutex<Bar>> = Arc::new(Mutex::new(
        BarBuilder::default()
            .desc("batch GTFS processing")
            .total(archive_paths.len())
            .animation("fillup")
            .build()
            .map_err(|e| {
                ScheduleError::InternalError(format!("failure building progress bar: {e}"))
            })?,
    ));

    let errors: Vec<ScheduleError> = archive_paths
        .iter()
        .enumerate()
        .collect_vec()
        .par_chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|(edge_list_offset, dir_entry)| {
                    if let Ok(mut bar) = bar.clone().lock() {
                        let _ = bar.update(1);
                    }
                    let path = dir_entry.path();
                    let bundle_file = path.to_str().ok_or_else(|| {
                        ScheduleError::GtfsAppError(format!(
                            "unable to convert directory entry into string: {dir_entry:?}"
                        ))
                    })?;
                    let edge_list_id = *start_edge_list_id + edge_list_offset;
                    process_bundle(
                        bundle_file,
                        &edge_list_id,
                        start_date,
                        end_date,
                        spatial_index.clone(),
                        missing_stop_location_policy,
                        distance_calculation_policy,
                        output_directory,
                        overwrite,
                    )
                    .map_err(|e| {
                        ScheduleError::GtfsAppError(format!("while processing {bundle_file}, {e}"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect_vec_list()
        .into_iter()
        .flat_map(|chunk| chunk.into_iter().flat_map(|r| r.err()))
        .collect_vec();

    eprintln!(); // end progress bar

    if !errors.is_empty() {
        Err(batch_processing_error(&errors))
    } else {
        Ok(())
    }
}

/// read a single GTFS archive and prepare a Compass EdgeList dataset from it.
/// trips with date outside of [start_date, end_date] are removed.
pub fn process_bundle(
    bundle_file: &str,
    edge_list_id: &usize,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    spatial_index: Arc<SpatialIndex>,
    missing_stop_location_policy: &MissingStopLocationPolicy,
    distance_calculation_policy: &DistanceCalculationPolicy,
    output_directory: &Path,
    overwrite: bool,
) -> Result<(), ScheduleError> {
    let gtfs = Gtfs::new(bundle_file).map_err(ScheduleError::from)?;

    let metadata = json! [{
        "agencies": json![&gtfs.agencies],
        "feed_info": json![&gtfs.feed_info],
        "read_duration": json![&gtfs.read_duration],
        "calendar": json![&gtfs.calendar],
        "calendar_dates": json![&gtfs.calendar_dates]
    }];
    let metadata_str = serde_json::to_string_pretty(&metadata).map_err(|e| {
        ScheduleError::GtfsAppError(format!("failure writing GTFS Agencies as JSON string: {e}"))
    })?;

    let gtfs_arc = Arc::new(gtfs);

    // for O(1) lookup of Addition/Deletion in calendar_dates.txt by (service_id, date)
    let gtfs_dates_lookup: Option<HashMap<String, HashMap<NaiveDate, Exception>>> =
        if gtfs_arc.calendar_dates.is_empty() {
            None
        } else {
            let lookup = gtfs_arc
                .calendar_dates
                .iter()
                .map(|(service_id, dates)| {
                    let inner = dates
                        .iter()
                        .map(|d| (d.date.clone(), d.exception_type.clone()))
                        .collect::<HashMap<_, _>>();
                    (service_id.clone(), inner)
                })
                .collect::<HashMap<_, _>>();
            Some(lookup)
        };

    // Get ordered StopTimes, RouteID and start_dates for each trip that intersects the dates
    let mut trip_stop_times: HashMap<String, Vec<StopTime>> = HashMap::new();
    let mut trip_start_dates: HashMap<String, NaiveDate> = HashMap::new();
    let mut trip_routes: HashMap<String, String> = HashMap::new();

    for (trip_id, trip) in gtfs_arc.clone().trips.iter() {
        let intersection_start_date_opt = find_trip_start_date(
            trip,
            gtfs_arc.clone(),
            gtfs_dates_lookup.as_ref(),
            start_date,
            end_date,
        )?;

        if let Some(intersection_start_date) = intersection_start_date_opt {
            trip_stop_times.insert(trip_id.clone(), get_ordered_stops(trip));
            trip_routes.insert(trip_id.clone(), trip.route_id.to_owned());
            trip_start_dates.insert(trip_id.clone(), intersection_start_date);
        }
    }

    // Pre-compute lat,lon location of all stops
    // with `get_stop_location` which returns the lat,lon
    // or the parent's lat,lon if available
    let stop_locations: HashMap<String, Option<Point<f64>>> = gtfs_arc
        .stops
        .iter()
        .map(|(stop_id, stop)| {
            (
                stop_id.clone(),
                get_stop_location(stop.clone(), gtfs_arc.clone()),
            )
        })
        .collect();

    // Construct edge lists
    let mut edge_id: usize = 0;
    let mut edges: HashMap<(usize, usize), Edge> = HashMap::new();
    let mut schedules: HashMap<(usize, usize), Vec<ScheduleConfig>> = HashMap::new();
    for (trip_id, stop_times) in trip_stop_times {
        for (src, dst) in stop_times.windows(2).map(|w| (&w[0], &w[1])) {
            let src_compass: usize;
            let dst_compass: usize;
            let src_point: Point<f64>;
            let dst_point: Point<f64>;

            // Since `stop_locations` is computed from `gtfs.stops`, this should never fail
            let maybe_src = stop_locations.get(&src.stop.id).ok_or_else(|| {
                ScheduleError::MalformedGtfsError(format!(
                    "source stop_id '{}' is not associated with a geographic location in either it's stop row or any parent row (see 'parent_station' of GTFS Stops.txt)",
                    src.stop.id
                ))
            })?;
            let maybe_dst = stop_locations.get(&dst.stop.id).ok_or_else(|| {
                ScheduleError::MalformedGtfsError(format!(
                    "destination stop_id '{}' is not associated with a geographic location in either it's stop row or any parent row (see 'parent_station' of GTFS Stops.txt)",
                    dst.stop.id
                ))
            })?;

            if let (Some(src_point_), Some(dst_point_)) = (maybe_src, maybe_dst) {
                // If you can find both:
                // Map to closest compass vertex
                src_compass = match_closest_graph_id(src_point_, spatial_index.clone())?;
                dst_compass = match_closest_graph_id(dst_point_, spatial_index.clone())?;

                // These points are used to compute the distance
                // Should we instead be using the graph node?
                // For instance, what happens if src_compass == dst_compass?
                src_point = src_point_.to_owned();
                dst_point = dst_point_.to_owned();
            } else {
                // If any is missing:
                match missing_stop_location_policy {
                    MissingStopLocationPolicy::Fail => {
                        return Err(ScheduleError::MissingStopLocationAndParentError(format!(
                            "{} or {}",
                            src.stop.id, dst.stop.id
                        )))
                    }
                    MissingStopLocationPolicy::DropStop => continue,
                }
            }

            // This only gets to run if all previous conditions are met
            if let std::collections::hash_map::Entry::Vacant(e) =
                edges.entry((src_compass, dst_compass))
            {
                // Estimate distance
                let distance: Length = match distance_calculation_policy {
                    DistanceCalculationPolicy::Haversine => compute_haversine(src_point, dst_point),
                    DistanceCalculationPolicy::Shape => todo!(),
                    DistanceCalculationPolicy::Fallback => todo!(),
                };

                let edge = Edge::new(*edge_list_id, edge_id, src_compass, dst_compass, distance);
                e.insert(edge);
                schedules.insert((src_compass, dst_compass), vec![]);
                edge_id += 1;
            }

            // Pick departure OR arrival time
            let raw_src_departure_time = match (src.departure_time, src.arrival_time) {
                (Some(departure), _) => Ok(departure),
                (None, Some(arrival)) => Ok(arrival),
                (None, None) => Err(ScheduleError::MissingAllStopTimesError(src.stop.id.clone())),
            }?;
            let raw_dst_arrival_time = match (dst.arrival_time, dst.departure_time) {
                (Some(arrival), _) => Ok(arrival),
                (None, Some(departure)) => Ok(departure),
                (None, None) => Err(ScheduleError::MissingAllStopTimesError(src.stop.id.clone())),
            }?;

            // The deserialization of Gtfs is in non-negative seconds (`deserialize_optional_time`)
            let start_date = trip_start_dates.get(&trip_id).ok_or_else(|| {
                ScheduleError::MalformedGtfsError(format!(
                    "calendar.txt missing entry for trip_id '{trip_id}' found in stop_times.txt"
                ))
            })?;
            let src_departure_offset = Duration::seconds(raw_src_departure_time as i64);
            let src_departure_time = start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(src_departure_offset)
                })
                .ok_or_else(|| {
                    let start_str = start_date.format("%m-%d-%Y");
                    let msg = format!("appending departure offset '{src_departure_offset}' to start_date '{start_str}' produced an empty result (invalid combination)");
                    ScheduleError::InvalidDataError(msg)
                })?;

            let dst_departure_offset = Duration::seconds(raw_dst_arrival_time as i64);
            let dst_arrival_time = start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(dst_departure_offset)
                })
                .ok_or_else(|| {
                    let start_str = start_date.format("%m-%d-%Y");
                    let msg = format!("appending departure offset '{dst_departure_offset}' to start_date '{start_str}' produced an empty result (invalid combination)");
                    ScheduleError::InvalidDataError(msg)
                })?;

            let route_id = trip_routes
                .get(&trip_id)
                .ok_or_else(|| {
                    ScheduleError::MalformedGtfsError(format!(
                        "trip '{trip_id}' has no associated route_id"
                    ))
                })?
                .to_owned();
            let schedule = ScheduleConfig {
                edge_id,
                src_departure_time,
                dst_arrival_time,
                route_id,
            };
            schedules
                .get_mut(&(src_compass, dst_compass))
                .ok_or_else(||{
                    ScheduleError::InternalError(format!("expected relation ({src_compass})->({dst_compass}) not created in 'schedules' collection"))
                })?
                .push(schedule);
        }
    }

    // Check consistent dictionary keys
    let edge_keys: HashSet<_> = edges.keys().collect();
    if edge_keys != schedules.keys().collect() {
        return Err(ScheduleError::InvalidResultKeysError);
    }

    // Write to files
    let metadata_filename = format!("edges-gtfs-metadata-{edge_list_id}.json");
    std::fs::write(output_directory.join(metadata_filename), &metadata_str).map_err(|e| {
        ScheduleError::GtfsAppError(format!("failed writing GTFS Agency metadata: {e}"))
    })?;
    let edges_filename = format!("edges-compass-{edge_list_id}.csv.gz");
    let schedules_filename = format!("edges-schedules-{edge_list_id}.csv.gz");
    let mut edges_writer = create_writer(
        output_directory,
        &edges_filename,
        true,
        QuoteStyle::Necessary,
        overwrite,
    );
    let mut schedules_writer = create_writer(
        output_directory,
        &schedules_filename,
        true,
        QuoteStyle::Necessary,
        overwrite,
    );

    for k in edge_keys {
        let edge = edges.get(k).ok_or(ScheduleError::InternalError(format!(
            "edge {k:?} not present in 'edges' array"
        )))?;
        let schedule_vec: &Vec<ScheduleConfig> =
            schedules
                .get(k)
                .ok_or(ScheduleError::InternalError(format!(
                    "edge {k:?} not present in 'schedules' array"
                )))?;

        if let Some(ref mut writer) = edges_writer {
            let edge_config = EdgeConfig {
                edge_id: edge.edge_id,
                src_vertex_id: edge.src_vertex_id,
                dst_vertex_id: edge.dst_vertex_id,
                distance: edge.distance.get::<uom::si::length::meter>(),
            };
            writer.serialize(edge_config).map_err(|e| {
                ScheduleError::GtfsAppError(format!(
                    "Failed to write to edges file {}: {}",
                    String::from(&edges_filename),
                    e
                ))
            })?;
        }

        if let Some(ref mut writer) = schedules_writer {
            for schedule in schedule_vec.iter() {
                writer.serialize(schedule).map_err(|e| {
                    ScheduleError::GtfsAppError(format!(
                        "Failed to write to schedules file {}: {}",
                        String::from(&schedules_filename),
                        e
                    ))
                })?;
            }
        }
    }
    Ok(())
}

// Checks the stop and its parent for lon,lat location. Returns None if this fails (parent doesn't exists or doesn't have location)
fn get_stop_location(stop: Arc<Stop>, gtfs: Arc<Gtfs>) -> Option<Point<f64>> {
    // Happy path, we have the info in this point
    // lon,lat is required if `location_type` in [0, 1, 2]
    if let (Some(lon), Some(lat)) = (stop.longitude, stop.latitude) {
        return Some(Point::new(lon, lat));
    }

    // Use lon,lat from parent station if data is missing. `parent_station` is required for `location_type=3 or 4`
    //
    // This could be done recursively but I think fixing it to
    // look only one step further is better. If this doesn't work
    // there are some wrong assumptions about the data
    stop.parent_station
        .clone()
        .and_then(|parent_id| gtfs.stops.get(&parent_id))
        .and_then(
            |parent_stop| match (parent_stop.longitude, parent_stop.latitude) {
                (Some(lon), Some(lat)) => Some(Point::new(lon, lat)),
                _ => None,
            },
        )
}

/// helper function for map matching stop locations to the graph.
fn match_closest_graph_id(
    point: &Point<f64>,
    spatial_index: Arc<SpatialIndex>,
) -> Result<usize, ScheduleError> {
    let _point = Point::new(point.x() as f32, point.y() as f32);

    // This fails if: 1) The spatial index fails, or 2) it returns an edge
    let nearest_result = spatial_index.nearest_graph_id(&_point)?;
    match nearest_result {
        NearestSearchResult::NearestVertex(vertex_id) => Ok(vertex_id.0),
        _ => Err(ScheduleError::GtfsAppError(format!(
            "could not find matching vertex for point {} in spatial index. consider expanding the distance tolerance or allowing for stop filtering.",
            point.to_wkt()
        ))),
    }
}

/// helper function to extract the calendar for a given trip.
fn get_trip_calendar(trip: &Trip, gtfs: Arc<Gtfs>) -> Result<Box<Calendar>, ScheduleError> {
    let calendar = gtfs
        .get_calendar(&trip.service_id)
        .map_err(|e| ScheduleError::InvalidCalendarError(format!("{e}")))?;

    Ok(Box::from(calendar.clone()))
}

/// uses calendar.txt and calendar_dates.txt to test if a given Trip runs within the
/// time range [start_date, end_date].
fn find_trip_start_date(
    trip: &Trip,
    gtfs: Arc<Gtfs>,
    dates: Option<&HashMap<String, HashMap<NaiveDate, Exception>>>,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let calendar = gtfs
        .get_calendar(&trip.service_id)
        .map_err(|e| ScheduleError::InvalidCalendarError(format!("{e}")));
    match (calendar, dates) {
        // archive contains both calendar.txt and calendar_dates.txt file, so we have to consider date exceptions
        (Ok(c), Some(cd)) => {
            let in_calendar = (c.start_date <= *end_date) && (*start_date <= c.end_date);
            // only test calendar dates within dates supported by both calendar + user arguments
            let query_start = std::cmp::max(*start_date, c.start_date);
            let query_end = std::cmp::min(*end_date, c.end_date);
            search_calendar_dates(&query_start, &query_end, in_calendar, &trip.service_id, cd)
        }

        // archive only contains calendar.txt, so we are looking for an intersection of two date ranges
        (Ok(c), None) => Ok(search_calendar(start_date, end_date, c)),

        // archive only contains calendar_dates.txt, so we are looking for a single addition that matches our date range
        (Err(_), Some(cd)) => {
            search_calendar_dates(start_date, end_date, false, &trip.service_id, cd)
        }

        (Err(_), None) => {
            let msg = format!("trip_id '{}' with service_id '{}' has no entry in either calendar.txt or calendar_dates.txt", trip.id, trip.service_id);
            Err(ScheduleError::MalformedGtfsError(msg))
        }
    }
}

fn search_calendar(
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    calendar: &Calendar,
) -> Option<NaiveDate> {
    let query_start = std::cmp::max(*start_date, calendar.start_date);
    let query_end = std::cmp::min(*end_date, calendar.end_date);
    if query_end > query_start {
        None
    } else {
        Some(query_start)
    }
}

/// helper function to test existence of an Exception within a date range.
/// the behavior of what we do when we encounter exceptions depends on if
/// our date range [start_date, end_date] was found to match the service in
/// calendar.txt.
///
/// terminates early for any of these 3 cases:
///   - case 1: date range is NOT in calendar.txt, but we found one matching date addition
///   - case 2: date range IS in calendar.txt, and we found one date without an exception
///   - case 3: date range IS in calendar.txt, and we found one date with an addition
///     - this case could also be an Error, but we count it here as just a redundancy
fn search_calendar_dates(
    query_start: &NaiveDate,
    query_end: &NaiveDate,
    date_range_in_calendar: bool,
    service_id: &str,
    dates: &HashMap<String, HashMap<NaiveDate, Exception>>,
) -> Result<Option<NaiveDate>, ScheduleError> {
    let mut current_date = query_start.clone();
    while &current_date <= query_end {
        // if date range not in calendar, we are looking for _one_ addition in range
        // if date range in calendar, we are looking for _one_ date not deleted

        let date_lookup_opt = dates.get(service_id);
        let exception_opt = match date_lookup_opt {
            Some(lookup) => lookup.get(&current_date),
            None => None,
        };
        match (date_range_in_calendar, exception_opt) {
            (false, Some(Exception::Added)) => return Ok(Some(current_date.clone())), // case 1: found one addition, exit
            (true, None) => return Ok(Some(current_date.clone())), // case 2: not deleted or added <=> not deleted, exit
            (true, Some(Exception::Added)) => return Ok(Some(current_date.clone())), // case 3: redundancy/bad data, but exit
            _ => {}
        }

        let next_date = current_date.succ_opt().ok_or_else(|| {
            let msg = format!(
                "Date overflow in service coverage check. cursor: '{}', date range: [{},{}]",
                current_date.format("%m-%d-%Y"),
                query_start.format("%m-%d-%Y"),
                query_end.format("%m-%d-%Y"),
            );
            ScheduleError::MalformedGtfsError(msg)
        })?;
        current_date = next_date
    }
    Ok(None)
}

/// Returns an ordered (ascending) vector of [StopTime]. Internally uses [BinaryHeap] to sort. In order to return the
/// [BinaryHeap] itself, [StopTime] would need to implement [Ord].
fn get_ordered_stops(trip: &Trip) -> Vec<StopTime> {
    // Get ordered indices
    let stop_queue_order: BinaryHeap<(u32, usize)> = trip
        .stop_times
        .iter()
        .enumerate()
        .map(|(i, st)| (st.stop_sequence, i))
        .collect();

    // Map indices list into objects
    stop_queue_order
        .into_sorted_vec() // Ascending according to documentation
        .iter()
        .map(|(_, idx)| trip.stop_times[*idx].clone())
        .collect()
}

/// helper function to build a filewriter for writing either .csv.gz or
/// .txt.gz files for compass datasets while respecting the user's overwrite
/// preferences and properly formatting WKT outputs.
fn create_writer(
    directory: &Path,
    filename: &str,
    has_headers: bool,
    quote_style: QuoteStyle,
    overwrite: bool,
) -> Option<csv::Writer<GzEncoder<File>>> {
    let filepath = directory.join(filename);
    if filepath.exists() && !overwrite {
        return None;
    }
    let file = File::create(filepath).unwrap();
    let buffer = GzEncoder::new(file, Compression::default());
    let writer = csv::WriterBuilder::new()
        .has_headers(has_headers)
        .quote_style(quote_style)
        .from_writer(buffer);
    Some(writer)
}

#[cfg(test)]
mod test {
    use crate::schedule::bundle_ops::get_ordered_stops;
    use gtfs_structures::Gtfs;
    use std::path::PathBuf;

    #[test]
    fn test_stop_orders_by_stop_sequence() {
        // Load test gtfs
        let test_bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("boulder_co")
            .join("ucb-gtfs.zip");

        let gtfs = Gtfs::new(
            test_bundle
                .to_str()
                .unwrap_or_else(|| panic!("Failed to interpret {test_bundle:?} as string")),
        )
        .expect("Test bundle not found in boulder_co/ucb-gtfs.zip");

        // Check that all stops for all trips are in ascending order
        for (_, trip) in gtfs.trips {
            assert!(get_ordered_stops(&trip)
                .iter()
                .map(|st| st.stop_sequence)
                .collect::<Vec<u32>>()
                .is_sorted());
        }
    }
}
