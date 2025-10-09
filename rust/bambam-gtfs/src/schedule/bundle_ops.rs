use chrono::{Duration, NaiveDate, NaiveDateTime};
use csv::QuoteStyle;
use flate2::{write::GzEncoder, Compression};
use geo::Point;
use gtfs_structures::{Exception, Gtfs, Stop, StopTime};
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
    collections::{HashMap, HashSet},
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
    MissingStopLocationPolicy, ProcessedTrip,
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
    let chunk_size = archive_paths.len() / std::cmp::max(1, parallelism);

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
                .collect_vec()
        })
        .collect_vec_list()
        .into_iter()
        .flat_map(|chunks| {
            chunks
                .into_iter()
                .flat_map(|chunk| chunk.into_iter().flat_map(|r| r.err()))
        })
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
    let gtfs = Arc::new(Gtfs::new(bundle_file)?);

    // collect metadata for writing to file
    let metadata = json! [{
        "agencies": json![&gtfs.agencies],
        "feed_info": json![&gtfs.feed_info],
        "read_duration": json![&gtfs.read_duration],
        "calendar": json![&gtfs.calendar],
        "calendar_dates": json![&gtfs.calendar_dates],
        "route_ids": json![gtfs.routes.keys().collect_vec()]
    }];
    let metadata_str = serde_json::to_string_pretty(&metadata).map_err(|e| {
        ScheduleError::GtfsAppError(format!("failure writing GTFS Agencies as JSON string: {e}"))
    })?;

    // for O(1) lookup of Addition/Deletion in calendar_dates.txt by (service_id, date)
    let gtfs_dates_lookup: Option<HashMap<String, HashMap<NaiveDate, Exception>>> =
        if gtfs.calendar_dates.is_empty() {
            None
        } else {
            let lookup = gtfs
                .calendar_dates
                .iter()
                .map(|(service_id, dates)| {
                    let inner = dates
                        .iter()
                        .map(|d| (d.date, d.exception_type))
                        .collect::<HashMap<_, _>>();
                    (service_id.clone(), inner)
                })
                .collect::<HashMap<_, _>>();
            Some(lookup)
        };

    // get trips that match our date range
    let mut trips: HashMap<String, ProcessedTrip> = HashMap::new();
    for t in gtfs.trips.values() {
        let trip_data_opt =
            ProcessedTrip::new(t, &gtfs, gtfs_dates_lookup.as_ref(), start_date, end_date)?;
        if let Some(trip_data) = trip_data_opt {
            let _ = trips.insert(trip_data.trip_id.clone(), trip_data);
        }
    }
    if trips.is_empty() {
        let msg = format!(
            "date range [{}, {}] did not match any trips",
            start_date.format("%m-%d-%Y"),
            end_date.format("%m-%d-%Y"),
        );
        return Err(ScheduleError::GtfsAppError(msg));
    }

    // Pre-compute lat,lon location of all stops
    // with `get_stop_location` which returns the lat,lon
    // or the parent's lat,lon if available
    let stop_locations: HashMap<String, Option<Point<f64>>> = gtfs
        .stops
        .iter()
        .map(|(stop_id, stop)| {
            (
                stop_id.clone(),
                get_stop_location(stop.clone(), gtfs.clone()),
            )
        })
        .collect();

    // Construct edge lists
    let mut edge_id: usize = 0;
    let mut edges: HashMap<(usize, usize), Edge> = HashMap::new();
    let mut schedules: HashMap<(usize, usize), Vec<ScheduleConfig>> = HashMap::new();
    for trip in trips.values() {
        for (src, dst) in trip.stop_times.windows(2).map(|w| (&w[0], &w[1])) {
            let map_match_result = map_match(src, dst, &stop_locations, spatial_index.clone())?;
            let ((src_id, src_point), (dst_id, dst_point)) =
                match (map_match_result, missing_stop_location_policy) {
                    (Some(result), _) => result,
                    (None, MissingStopLocationPolicy::Fail) => {
                        let msg = format!("{} or {}", src.stop.id, dst.stop.id);
                        return Err(ScheduleError::MissingStopLocationAndParentError(msg));
                    }
                    (None, MissingStopLocationPolicy::DropStop) => continue,
                };

            // This only gets to run if all previous conditions are met
            // it adds the edge if it has not yet been added.
            edges.entry((src_id, dst_id)).or_insert_with(|| {
                // Estimate distance
                let distance: Length = match distance_calculation_policy {
                    DistanceCalculationPolicy::Haversine => compute_haversine(src_point, dst_point),
                    DistanceCalculationPolicy::Shape => todo!(),
                    DistanceCalculationPolicy::Fallback => todo!(),
                };

                let edge = Edge::new(*edge_list_id, edge_id, src_id, dst_id, distance);
                schedules.insert((src_id, dst_id), vec![]);
                edge_id += 1;
                edge
            });

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

            // // The deserialization of Gtfs is in non-negative seconds (`deserialize_optional_time`)
            let src_departure_offset = Duration::seconds(raw_src_departure_time as i64);
            let src_departure_time = trip.start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(src_departure_offset)
                })
                .ok_or_else(|| {
                    let start_str = trip.start_date.format("%m-%d-%Y");
                    let msg = format!("appending departure offset '{src_departure_offset}' to trip.start_date '{start_str}' produced an empty result (invalid combination)");
                    ScheduleError::InvalidDataError(msg)
                })?;

            let dst_departure_offset = Duration::seconds(raw_dst_arrival_time as i64);
            let dst_arrival_time = trip.start_date
                .and_hms_opt(0, 0, 0)
                .and_then(|datetime| {
                    datetime.checked_add_signed(dst_departure_offset)
                })
                .ok_or_else(|| {
                    let start_str = trip.start_date.format("%m-%d-%Y");
                    let msg = format!("appending departure offset '{dst_departure_offset}' to start_date '{start_str}' produced an empty result (invalid combination)");
                    ScheduleError::InvalidDataError(msg)
                })?;

            let schedule = ScheduleConfig {
                edge_id,
                src_departure_time,
                dst_arrival_time,
                route_id: trip.route_id.clone(),
            };
            schedules
                .get_mut(&(src_id, dst_id))
                .ok_or_else(||{
                    ScheduleError::InternalError(format!("expected relation ({src_id})->({dst_id}) not created in 'schedules' collection"))
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
    std::fs::create_dir_all(output_directory).map_err(|e| {
        let outdir = output_directory.to_str().unwrap_or_default();
        ScheduleError::GtfsAppError(format!(
            "unable to create output directory path '{outdir}': {e}"
        ))
    })?;
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

pub type MapMatchResult = ((usize, Point<f64>), (usize, Point<f64>));
/// finds the vertex and point associated with src and dst StopTime entry.
///
/// # Result
///
/// the source and destination, each a tuple of (VertexId, Coordinate)
fn map_match(
    src: &StopTime,
    dst: &StopTime,
    stop_locations: &HashMap<String, Option<Point<f64>>>,
    spatial_index: Arc<SpatialIndex>,
) -> Result<Option<MapMatchResult>, ScheduleError> {
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

    match (maybe_src, maybe_dst) {
        (Some(src_point_), Some(dst_point_)) => {
            // If you can find both:
            // Map to closest compass vertex
            let src_compass = match_closest_graph_id(src_point_, spatial_index.clone())?;
            let dst_compass = match_closest_graph_id(dst_point_, spatial_index.clone())?;

            // These points are used to compute the distance
            // Should we instead be using the graph node?
            // For instance, what happens if src_compass == dst_compass?
            let src_point = src_point_.to_owned();
            let dst_point = dst_point_.to_owned();
            Ok(Some(((src_compass, src_point), (dst_compass, dst_point))))
        }
        _ => Ok(None),
    }
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
