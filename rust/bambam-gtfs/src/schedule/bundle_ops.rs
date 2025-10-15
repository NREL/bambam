use chrono::{Duration, NaiveDate};
use csv::QuoteStyle;
use flate2::{write::GzEncoder, Compression};
use geo::{LineString, Point};
use gtfs_structures::{Gtfs, Stop, StopTime};
use itertools::Itertools;
use kdam::{Bar, BarBuilder, BarExt};
use rayon::prelude::*;
use routee_compass_core::model::{
    map::{NearestSearchResult, SpatialIndex},
    network::{EdgeConfig, EdgeId, VertexId},
};
use serde_json::json;
use std::{
    collections::HashMap,
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
    DateMappingPolicy, MissingStopLocationPolicy, ScheduleRow, SortedTrip,
};

/// configures the run of the GTFS import
#[derive(Clone)]
pub struct ProcessBundlesConfig {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub starting_edge_list_id: usize,
    pub spatial_index: Arc<SpatialIndex>,
    pub missing_stop_location_policy: MissingStopLocationPolicy,
    pub distance_calculation_policy: DistanceCalculationPolicy,
    pub date_mapping_policy: DateMappingPolicy,
    pub output_directory: String,
    pub overwrite: bool,
}

pub struct GtfsBundle {
    pub edges: Vec<GtfsEdge>,
    pub metadata: serde_json::Value,
}

pub struct GtfsEdge {
    edge: EdgeConfig,
    geometry: LineString,
    schedules: Vec<ScheduleRow>,
}

impl GtfsEdge {
    pub fn new(edge: EdgeConfig, geometry: LineString) -> Self {
        Self {
            edge,
            geometry,
            schedules: vec![],
        }
    }

    pub fn add_schedule(&mut self, schedule: ScheduleRow) {
        self.schedules.push(schedule);
    }
}

/// multithreaded GTFS processing.
///
/// # Arguments
///
/// * `bundle_directory_path` - location of zipped GTFS archives
/// * `parallelism` - threads dedicated to GTFS import
/// * `conf` - configuration for processing, see for options
/// * `ignore_bad_gtfs` - if true, any failed processing does not terminate import and
///   remaining archives are processed into edge list outputs. errors are logged.
///
pub fn batch_process(
    bundle_directory_path: &Path,
    parallelism: usize,
    conf: Arc<ProcessBundlesConfig>,
    ignore_bad_gtfs: bool,
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

    let (bundles, errors): (Vec<GtfsBundle>, Vec<ScheduleError>) = archive_paths
        // .iter()
        // .enumerate()
        // .collect_vec()
        .par_chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|dir_entry| {
                    if let Ok(mut bar) = bar.clone().lock() {
                        let _ = bar.update(1);
                    }
                    let path = dir_entry.path();
                    let bundle_file = path.to_str().ok_or_else(|| {
                        ScheduleError::GtfsAppError(format!(
                            "unable to convert directory entry into string: {dir_entry:?}"
                        ))
                    })?;
                    // let edge_list_id = *start_edge_list_id + edge_list_offset;
                    process_bundle(bundle_file, conf.clone()).map_err(|e| {
                        ScheduleError::GtfsAppError(format!("while processing {bundle_file}, {e}"))
                    })
                })
                .collect_vec()
        })
        .collect_vec_list()
        .into_iter()
        .flat_map(|chunks| chunks.into_iter().flat_map(|chunk| chunk.into_iter()))
        .collect_vec()
        .into_iter()
        .partition_result();

    eprintln!(); // end progress bar

    // handle errors, either by terminating early, or, logging them
    if !errors.is_empty() && !ignore_bad_gtfs {
        return Err(batch_processing_error(&errors));
    } else if !errors.is_empty() {
        // log errors
        for error in errors {
            log::error!("{error}");
        }
    }

    // write results to file
    let (_, write_errors): (Vec<_>, Vec<_>) = bundles
        .into_iter()
        .enumerate()
        .collect_vec()
        .par_chunks(chunk_size)
        .map(|chunk| {
            chunk.iter().map(|(index, bundle)| {
                let edge_list_id = conf.starting_edge_list_id + index;
                write_bundle(bundle, conf.clone(), edge_list_id)
            })
        })
        .collect_vec_list()
        .into_iter()
        .flat_map(|chunks| {
            chunks
                .into_iter()
                .flat_map(|chunk| chunk.into_iter().filter(|r| r.is_err()).collect_vec())
        })
        .collect_vec()
        .into_iter()
        .partition_result();

    if !write_errors.is_empty() {
        Err(batch_processing_error(&write_errors))
    } else {
        Ok(())
    }
}

/// read a single GTFS archive and prepare a Compass EdgeList dataset from it.
/// trips with date outside of [start_date, end_date] are removed.
pub fn process_bundle(
    bundle_file: &str,
    c: Arc<ProcessBundlesConfig>,
) -> Result<GtfsBundle, ScheduleError> {
    let gtfs = Arc::new(Gtfs::new(bundle_file)?);

    // get trips that match our date range
    let mut trips: HashMap<String, SortedTrip> = HashMap::new();
    for t in gtfs.trips.values() {
        let trip_data_opt = SortedTrip::new(t)?;
        if let Some(trip_data) = trip_data_opt {
            let _ = trips.insert(trip_data.trip_id.clone(), trip_data);
        }
    }
    if trips.is_empty() {
        let msg = format!(
            "date range [{}, {}] did not match any trips",
            c.start_date.format("%m-%d-%Y"),
            c.end_date.format("%m-%d-%Y"),
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
    let mut edge_id: EdgeId = EdgeId(0);
    let mut edges: HashMap<(VertexId, VertexId), GtfsEdge> = HashMap::new();
    let mut date_mapping: HashMap<String, HashMap<NaiveDate, NaiveDate>> = HashMap::new();
    for target_date in c.date_mapping_policy.iter() {
        for trip in trips.values() {
            // apply date mapping
            let picked_date = c
                .date_mapping_policy
                .pick_date(&target_date, trip, gtfs.clone())?;
            if target_date != picked_date && !date_mapping.contains_key(&trip.route_id) {
                // date mapping is organized by ServiceId, but our TraversalModel expects RouteId
                date_mapping
                    .entry(trip.route_id.clone())
                    .and_modify(|dates| {
                        dates.insert(target_date, picked_date);
                    })
                    .or_insert(HashMap::from([(target_date, picked_date)]));
            }

            for (src, dst) in trip.stop_times.windows(2).map(|w| (&w[0], &w[1])) {
                let map_match_result =
                    map_match(src, dst, &stop_locations, c.spatial_index.clone())?;
                let ((src_id, src_point), (dst_id, dst_point)) =
                    match (map_match_result, &c.missing_stop_location_policy) {
                        (Some(result), _) => result,
                        (None, MissingStopLocationPolicy::Fail) => {
                            let msg = format!("{} or {}", src.stop.id, dst.stop.id);
                            return Err(ScheduleError::MissingStopLocationAndParentError(msg));
                        }
                        (None, MissingStopLocationPolicy::DropStop) => continue,
                    };

                // This only gets to run if all previous conditions are met
                // it adds the edge if it has not yet been added.
                let gtfs_edge = edges.entry((src_id, dst_id)).or_insert_with(|| {
                    let geometry = match &c.distance_calculation_policy {
                        DistanceCalculationPolicy::Haversine => {
                            LineString::new(vec![src_point.0, dst_point.0])
                        }
                        DistanceCalculationPolicy::Shape => todo!(),
                        DistanceCalculationPolicy::Fallback => todo!(),
                    };

                    // Estimate distance
                    let distance: Length = match &c.distance_calculation_policy {
                        DistanceCalculationPolicy::Haversine => {
                            compute_haversine(src_point, dst_point)
                        }
                        DistanceCalculationPolicy::Shape => todo!(),
                        DistanceCalculationPolicy::Fallback => todo!(),
                    };

                    let edge = EdgeConfig {
                        edge_id,
                        src_vertex_id: src_id,
                        dst_vertex_id: dst_id,
                        distance: distance.get::<uom::si::length::meter>(),
                    };

                    let gtfs_edge = GtfsEdge::new(edge, geometry);

                    // NOTE: edge id update completed after creating this Edge
                    edge_id = EdgeId(edge_id.0 + 1);

                    gtfs_edge
                });

                // Pick departure OR arrival time
                let raw_src_departure_time = match (src.departure_time, src.arrival_time) {
                    (Some(departure), _) => Ok(departure),
                    (None, Some(arrival)) => Ok(arrival),
                    (None, None) => {
                        Err(ScheduleError::MissingAllStopTimesError(src.stop.id.clone()))
                    }
                }?;
                let raw_dst_arrival_time = match (dst.arrival_time, dst.departure_time) {
                    (Some(arrival), _) => Ok(arrival),
                    (None, Some(departure)) => Ok(departure),
                    (None, None) => {
                        Err(ScheduleError::MissingAllStopTimesError(src.stop.id.clone()))
                    }
                }?;

                // // The deserialization of Gtfs is in non-negative seconds (`deserialize_optional_time`)
                let src_departure_offset = Duration::seconds(raw_src_departure_time as i64);
                let src_departure_time = picked_date
                        .and_hms_opt(0, 0, 0)
                        .and_then(|datetime| {
                            datetime.checked_add_signed(src_departure_offset)
                        })
                        .ok_or_else(|| {
                            let picked_str = picked_date.format("%m-%d-%Y");
                            let msg = format!("appending departure offset '{src_departure_offset}' to picked_date '{picked_str}' produced an empty result (invalid combination)");
                            ScheduleError::InvalidDataError(msg)
                        })?;

                let dst_departure_offset = Duration::seconds(raw_dst_arrival_time as i64);
                let dst_arrival_time = picked_date
                        .and_hms_opt(0, 0, 0)
                        .and_then(|datetime| {
                            datetime.checked_add_signed(dst_departure_offset)
                        })
                        .ok_or_else(|| {
                            let picked_str = picked_date.format("%m-%d-%Y");
                            let msg = format!("appending departure offset '{dst_departure_offset}' to picked_date '{picked_str}' produced an empty result (invalid combination)");
                            ScheduleError::InvalidDataError(msg)
                        })?;

                let schedule = ScheduleRow {
                    edge_id: gtfs_edge.edge.edge_id.0,
                    src_departure_time,
                    dst_arrival_time,
                    route_id: trip.route_id.clone(),
                };
                gtfs_edge.add_schedule(schedule);
            }
        }
    }

    let edges_sorted = edges
        .into_values()
        .sorted_by_cached_key(|e| e.edge.edge_id)
        .collect_vec();

    let metadata = json! [{
        "agencies": json![&gtfs.agencies],
        "feed_info": json![&gtfs.feed_info],
        "read_duration": json![&gtfs.read_duration],
        "calendar": json![&gtfs.calendar],
        "calendar_dates": json![&gtfs.calendar_dates],
        "route_ids": json![gtfs.routes.keys().collect_vec()],
        "date_mapping": json![date_mapping]
    }];

    let result = GtfsBundle {
        edges: edges_sorted,
        metadata,
    };

    Ok(result)
}

/// writes the provided bundle to files enumerated by the provided edge_list_id.
pub fn write_bundle(
    bundle: &GtfsBundle,
    c: Arc<ProcessBundlesConfig>,
    edge_list_id: usize,
) -> Result<(), ScheduleError> {
    // Write to files
    let output_directory = Path::new(&c.output_directory);
    let metadata_filename = format!("edges-gtfs-metadata-{edge_list_id}.json");
    std::fs::create_dir_all(output_directory).map_err(|e| {
        let outdir = output_directory.to_str().unwrap_or_default();
        ScheduleError::GtfsAppError(format!(
            "unable to create output directory path '{outdir}': {e}"
        ))
    })?;
    let metadata_str = serde_json::to_string_pretty(&bundle.metadata).map_err(|e| {
        ScheduleError::GtfsAppError(format!("failure writing GTFS Agencies as JSON string: {e}"))
    })?;
    std::fs::write(output_directory.join(metadata_filename), &metadata_str).map_err(|e| {
        ScheduleError::GtfsAppError(format!("failed writing GTFS Agency metadata: {e}"))
    })?;
    let edges_filename = format!("edges-compass-{edge_list_id}.csv.gz");
    let schedules_filename = format!("edges-schedules-{edge_list_id}.csv.gz");
    let geometries_filename = format!("edges-geometries-enumerated-{edge_list_id}.txt.gz");
    let mut edges_writer = create_writer(
        output_directory,
        &edges_filename,
        true,
        QuoteStyle::Necessary,
        c.overwrite,
    );
    let mut schedules_writer = create_writer(
        output_directory,
        &schedules_filename,
        true,
        QuoteStyle::Necessary,
        c.overwrite,
    );
    let mut geometries_writer = create_writer(
        output_directory,
        &geometries_filename,
        false,
        QuoteStyle::Never,
        c.overwrite,
    );

    for GtfsEdge {
        edge,
        geometry,
        schedules,
    } in bundle.edges.iter()
    {
        if let Some(ref mut writer) = edges_writer {
            writer.serialize(edge).map_err(|e| {
                ScheduleError::GtfsAppError(format!(
                    "Failed to write to edges file {}: {}",
                    String::from(&edges_filename),
                    e
                ))
            })?;
        }

        if let Some(ref mut writer) = schedules_writer {
            for schedule in schedules.iter() {
                writer.serialize(schedule).map_err(|e| {
                    ScheduleError::GtfsAppError(format!(
                        "Failed to write to schedules file {}: {}",
                        String::from(&schedules_filename),
                        e
                    ))
                })?;
            }
        }

        if let Some(ref mut writer) = geometries_writer {
            writer
                .serialize(geometry.to_wkt().to_string())
                .map_err(|e| {
                    ScheduleError::GtfsAppError(format!(
                        "Failed to write to geometry file {}: {}",
                        String::from(&edges_filename),
                        e
                    ))
                })?;
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

pub type MapMatchResult = ((VertexId, Point<f64>), (VertexId, Point<f64>));
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
) -> Result<VertexId, ScheduleError> {
    let point_f32 = Point::new(point.x() as f32, point.y() as f32);

    // This fails if: 1) The spatial index fails, or 2) it returns an edge
    let nearest_result = spatial_index.nearest_graph_id(&point_f32)?;
    match nearest_result {
        NearestSearchResult::NearestVertex(vertex_id) => Ok(vertex_id),
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
