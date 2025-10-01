use chrono::{Duration, NaiveDate, NaiveDateTime};
use csv::QuoteStyle;
use flate2::{write::GzEncoder, Compression};
use geo::Point;
use gtfs_structures::{Calendar, Gtfs, Stop, StopTime, Trip};
use routee_compass_core::model::{
    map::{NearestSearchResult, SpatialIndex},
    network::{Edge, EdgeConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    fs::File,
    path::Path,
    sync::Arc,
};
use uom::si::f64::Length;

use crate::schedule::{
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
    let gtfs_arc = Arc::new(gtfs);

    // Get ordered StopTimes, RouteID and start_dates for each trip that intersects the dates
    let mut trip_stop_times: HashMap<String, Vec<StopTime>> = HashMap::new();
    let mut trip_start_dates: HashMap<String, NaiveDate> = HashMap::new();
    let mut trip_routes: HashMap<String, String> = HashMap::new();

    for (trip_id, trip) in gtfs_arc.clone().trips.iter() {
        let trip_calendar = get_trip_calendar(trip, gtfs_arc.clone())?;
        let trip_intersects =
            (trip_calendar.start_date < *end_date) && (*start_date < trip_calendar.end_date);

        if trip_intersects {
            trip_stop_times.insert(trip_id.clone(), get_ordered_stops(trip));
            trip_routes.insert(trip_id.clone(), trip.route_id.to_owned());
            trip_start_dates.insert(trip_id.clone(), trip_calendar.start_date);
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
            let maybe_src = stop_locations.get(&src.stop.id).unwrap_or_else(|| {
                panic!(
                    "Attempted to get location for non existing stop: {}",
                    src.stop.id
                )
            });
            let maybe_dst = stop_locations.get(&dst.stop.id).unwrap_or_else(|| {
                panic!(
                    "Attempted to get location for non existing stop: {}",
                    dst.stop.id
                )
            });

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
            let start_date = trip_start_dates
                .get(&trip_id)
                .expect("Attempted to get starting date of non existing trip");
            let src_departure_time = start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(Duration::seconds(raw_src_departure_time as i64))
                })
                .ok_or(ScheduleError::OtherError(
                    "Invalid Datetime from Date".to_string(),
                ))?;

            let dst_arrival_time = start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(Duration::seconds(raw_dst_arrival_time as i64))
                })
                .ok_or(ScheduleError::OtherError(
                    "Invalid Datetime from Date".to_string(),
                ))?;

            let schedule = ScheduleConfig {
                edge_id,
                src_departure_time,
                dst_arrival_time,
                route_id: trip_routes
                    .get(&trip_id)
                    .expect("Attemted to get route of non existing trip")
                    .to_owned(),
            };
            schedules
                .get_mut(&(src_compass, dst_compass))
                .expect("Attempted to append to schedule for non existing edge")
                .push(schedule);
        }
    }

    // Check consistent dictionary keys
    let edge_keys: HashSet<_> = edges.keys().collect();
    if edge_keys != schedules.keys().collect() {
        return Err(ScheduleError::InvalidResultKeysError);
    }

    // Write to files
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
        let edge = edges.get(k).ok_or(ScheduleError::OtherError(
            "Edge key not present in edges array".to_string(),
        ))?;
        let schedule_vec: &Vec<ScheduleConfig> = schedules.get(k).ok_or(
            ScheduleError::OtherError("Edge key not present in schedules array".to_string()),
        )?;

        if let Some(ref mut writer) = edges_writer {
            let edge_config = EdgeConfig {
                edge_id: edge.edge_id,
                src_vertex_id: edge.src_vertex_id,
                dst_vertex_id: edge.dst_vertex_id,
                distance: edge.distance.get::<uom::si::length::meter>(),
            };
            writer.serialize(edge_config).map_err(|e| {
                ScheduleError::OtherError(format!(
                    "Failed to write to edges file {}: {}",
                    String::from(&edges_filename),
                    e
                ))
            })?;
        }

        if let Some(ref mut writer) = schedules_writer {
            for schedule in schedule_vec.iter() {
                writer.serialize(schedule).map_err(|e| {
                    ScheduleError::OtherError(format!(
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

fn match_closest_graph_id(
    point: &Point<f64>,
    spatial_index: Arc<SpatialIndex>,
) -> Result<usize, ScheduleError> {
    let _point = Point::new(point.x() as f32, point.y() as f32);

    // This fails if: 1) The spatial index fails, or 2) it returns an edge
    match spatial_index
        .nearest_graph_id(&_point)
        .map_err(|e| ScheduleError::SpatialIndexMapError { source: e })?
    {
        NearestSearchResult::NearestVertex(vertex_id) => Ok(vertex_id.0),
        _ => Err(ScheduleError::SpatialIndexIncorrectMapError),
    }
}

fn get_trip_calendar(trip: &Trip, gtfs: Arc<Gtfs>) -> Result<Box<Calendar>, ScheduleError> {
    let calendar = gtfs
        .get_calendar(&trip.service_id)
        .map_err(|e| ScheduleError::InvalidCalendarError(format!("{e}")))?;

    Ok(Box::from(calendar.clone()))
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
        .expect("Test bundle not found in configuration/gtfs-test/gtfs-test.zip");

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
