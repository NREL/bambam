use chrono::NaiveDate;
use geo::Point;
use gtfs_structures::{Gtfs, Stop, StopTime, Trip};
use routee_compass_core::model::{
    map::{NearestSearchResult, SpatialIndex},
    network::Edge,
};
use std::{
    collections::{BinaryHeap, HashMap},
    sync::Arc,
};
use uom::si::f64::{Length, Time};

use crate::schedule::{
    distance_calculation_policy::{compute_haversine, DistanceCalculationPolicy},
    schedule_error::ScheduleError,
    MissingStopLocationPolicy,
};

pub fn process_bundle(
    bundle_file: &str,
    edge_list_id: usize,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    spatial_index: Arc<SpatialIndex>,
    missing_stop_location_policy: &MissingStopLocationPolicy,
    distance_calculation_policy: &DistanceCalculationPolicy,
) -> Result<(), ScheduleError> {
    // TODO: Filter stops by start and end date

    let gtfs = Gtfs::new(bundle_file).map_err(|e| ScheduleError::BundleReadError { source: e })?;
    let gtfs_arc = Arc::new(gtfs);

    let mut trip_stop_times: HashMap<String, Vec<StopTime>> = HashMap::new();
    for (trip_id, trip) in gtfs_arc.clone().trips.iter() {
        if trip_intersects_dates(trip, start_date, end_date, gtfs_arc.clone())?{
            trip_stop_times.insert(trip_id.clone(), get_ordered_stops(trip));
        }
    }

    // let trip_stop_times: HashMap<String, Vec<StopTime>> = gtfs_arc
    //     .trips
    //     .iter()
    //     .map(|(trip_id, trip)|
    //         if trip_intersects_dates(trip, start_date, end_date, gtfs_arc.clone()){
    //             Some((trip_id.to_owned(), get_ordered_stops(trip)))
    //         } else { None }
    //     )
    //     .collect::<Vec<Option<(String, Vec<StopTime>)>>>()
    //     .into_iter().flatten()
    //     .collect();

    // Pre-compute location of all stops
    // with `get_stop_location`, which returns the lat,lon
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
    let mut schedules: HashMap<(usize, usize), Vec<Time>> = HashMap::new();
    for (_trip_id, stop_times) in trip_stop_times {
        for (src, dst) in stop_times.windows(2).map(|w| (&w[0], &w[1])) {
            // A solution to the possibly missing stop location
            let src_compass: usize;
            let dst_compass: usize;
            let src_point: Point<f64>;
            let dst_point: Point<f64>;

            let maybe_src = stop_locations.get(&src.stop.id).expect(&format!(
                "Attempted to get location for non existing stop: {}",
                src.stop.id
            ));
            let maybe_dst = stop_locations.get(&dst.stop.id).expect(&format!(
                "Attempted to get location for non existing stop: {}",
                dst.stop.id
            ));

            // This if let either identifies the vertices or it fails
            if let (Some(src_point_), Some(dst_point_)) = (maybe_src, maybe_dst) {
                // If you can find both:
                // Map to closest compass vertex
                // TODO: What to do with missing lat,lon?
                src_compass = match_closest_graph_id(src_point_, spatial_index.clone())?;
                dst_compass = match_closest_graph_id(dst_point_, spatial_index.clone())?;
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

            // This only gets run if all previous conditions are met
            if !edges.contains_key(&(src_compass, dst_compass)) {
                // Estimate distance
                let distance: Length = match distance_calculation_policy {
                    DistanceCalculationPolicy::Haversine => compute_haversine(src_point, dst_point),
                    DistanceCalculationPolicy::Shape => todo!(),
                    DistanceCalculationPolicy::Fallback => todo!(),
                };

                let edge = Edge::new(edge_list_id, edge_id, src_compass, dst_compass, distance);
                edges.insert((src_compass, dst_compass), edge);
                edge_id += 1;
            }

            // Pick departure OR arrival time
            let raw_departure_time = match (src.departure_time, src.arrival_time) {
                (Some(departure), _) => Ok(departure),
                (None, Some(arrival)) => Ok(arrival),
                (None, None) => Err(ScheduleError::MissingAllStopTimesError(src.stop.id.clone())),
            };

            // The deserialization of Gtfs is in non-negative seconds
            let departure_time = Time::new::<uom::si::time::second>(raw_departure_time? as f64);
            schedules
                .get_mut(&(src_compass, dst_compass))
                .expect("Attempted to append to schedule for non existing edge")
                .push(departure_time);
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

fn trip_intersects_dates(
    trip: &Trip,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    gtfs: Arc<Gtfs>,
) -> Result<bool, ScheduleError> {
    let calendar = gtfs
        .get_calendar(&trip.service_id)
        .map_err(|e| ScheduleError::InvalidCalendar(format!("{}", e)))?;

    Ok((calendar.start_date.clone() < end_date.clone())
        && (start_date.clone() < calendar.end_date.clone()))
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
            .join("configuration")
            .join("gtfs-test")
            .join("gtfs-test.zip");

        let gtfs = Gtfs::new(
            test_bundle
                .to_str()
                .expect(&format!("Failed to interpret {:?} as string", test_bundle)),
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
