use std::collections::HashMap;

use gtfs_structures::{RawGtfs, RawTrip};

pub type StopId = usize;

pub struct RawGtfsWrapper<'a> {
    pub filename: String,
    pub gtfs: &'a RawGtfs,
    pub stop_from_vertex_id: Vec<StopId>,
}

impl<'a> RawGtfsWrapper<'a> {
    // pub fn new(filename: String, gtfs: &'a RawGtfs) -> RawGtfsWrapper<'a> {
    //     RawGtfsWrapper { filename, gtfs }
    // }

    pub fn get_trip(&self, trip_id: usize) -> Result<&'a RawTrip, String> {
        let trips: &[RawTrip] = self
            .gtfs
            .trips
            .as_ref()
            .map_err(|e| format!("gtfs {} does not have trips due to: {}", self.filename, e))?;
        let trip = trips
            .get(trip_id)
            .ok_or_else(|| format!("gtfs {} does not have trip_id {}", self.filename, trip_id))?;
        Ok(trip)
    }
}
