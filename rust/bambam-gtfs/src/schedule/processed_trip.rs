use std::collections::{BinaryHeap, HashMap};

use chrono::NaiveDate;
use gtfs_structures::{Exception, Gtfs, StopTime, Trip};

use crate::schedule::{date_ops, schedule_error::ScheduleError};

/// a trip that matches our user's date range, prepared for edge list processing.
pub struct ProcessedTrip {
    /// GTFS trip identifier
    pub trip_id: String,
    /// GTFS route_id associated with this [`Trip`]
    pub route_id: String,
    /// service associated with this trip
    pub service_id: String,
    /// list of [`StopTime`] values associated with this [`Trip`] in sorted order
    pub stop_times: Vec<StopTime>,
    // /// starting date of this trip.
    // pub start_date: NaiveDate,
}

impl ProcessedTrip {
    /// creates a new trip data collection organized around generating scheduled edges
    /// in the Compass edge list.
    ///
    /// if this trip's date does not match the user date range, [`ProcessedTrip`] is not created.
    pub fn new(
        trip: &Trip,
        gtfs: &Gtfs,
        dates_lookup: Option<&HashMap<String, HashMap<NaiveDate, Exception>>>,
        // start_date: &NaiveDate,
        // end_date: &NaiveDate,
    ) -> Result<Option<ProcessedTrip>, ScheduleError> {
        let stop_times = get_ordered_stops(trip)?;
        let result = Self {
            trip_id: trip.id.clone(),
            route_id: trip.route_id.clone(),
            service_id: trip.service_id.clone(),
            stop_times,
            // start_date,
        };
        Ok(Some(result))
        // check for the "start date" that we can use to match
        // let intersection_start_date_opt =
        //     date_ops::find_trip_start_date(trip, gtfs, dates_lookup, start_date, end_date)?;
        // match intersection_start_date_opt {
        //     None => Ok(None),
        //     Some(start_date) => {
        //         let stop_times = get_ordered_stops(trip)?;
        //         let result = Self {
        //             trip_id: trip.id.clone(),
        //             route_id: trip.route_id.clone(),
        //             service_id: trip.service_id.clone(),
        //             stop_times,
        //             start_date,
        //         };
        //         Ok(Some(result))
        //     }
        // }
    }
}

/// Returns an ordered (ascending) vector of [StopTime]. Internally uses [BinaryHeap] to sort. In order to return the
/// [BinaryHeap] itself, [StopTime] would need to implement [Ord].
fn get_ordered_stops(trip: &Trip) -> Result<Vec<StopTime>, ScheduleError> {
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
        .map(|(_, idx)| {
            trip.stop_times.get(*idx).cloned().ok_or_else(|| {
                let msg = format!("expected stop index {idx} not found in trip {}", trip.id);
                ScheduleError::MalformedGtfsError(msg)
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod test {
    use super::get_ordered_stops;
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
            let result = get_ordered_stops(&trip).expect("should not fail");
            assert!(result
                .iter()
                .map(|st| st.stop_sequence)
                .collect::<Vec<u32>>()
                .is_sorted());
        }
    }
}
