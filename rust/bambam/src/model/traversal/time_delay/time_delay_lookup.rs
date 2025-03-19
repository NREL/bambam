use super::{delay_aggregation_type::DelayAggregationType, time_delay_record::TimeDelayRecord};
use crate::util::geo_utils;
use geo::{Geometry, Point};
use kdam::Bar;
use routee_compass_core::{
    config::{CompassConfigurationError, ConfigJsonExtensions},
    model::{
        traversal::TraversalModelError,
        unit::{Time, TimeUnit},
    },
    util::fs::read_utils,
};
use rstar::{RTree, AABB};
use std::path::Path;

pub struct TimeDelayLookup {
    lookup: RTree<TimeDelayRecord>,
    time_unit: TimeUnit,
    agg: DelayAggregationType,
}

impl TimeDelayLookup {
    /// builds a new lookup function for zonal time delays at either trip departure or arrival
    ///
    /// # Arguments
    ///
    /// * `lookup_file` - file containing geometries tagged with time values
    /// * `access_type` - "departure" or "arrival" data
    /// * `time_unit`   - time unit in source data
    ///
    /// # Returns
    ///
    /// * an object that can be used to lookup time delay values, or an error
    pub fn new(
        lookup_file: &Path,
        time_unit: TimeUnit,
        agg: DelayAggregationType,
    ) -> Result<TimeDelayLookup, TraversalModelError> {
        let data: Box<[TimeDelayRecord]> = read_utils::from_csv(
            &lookup_file,
            true,
            Some(Bar::builder().desc("time delay lookup")),
            None,
        )
        .map_err(|e| {
            TraversalModelError::BuildError(format!(
                "failure reading time delay rows from {}: {}",
                lookup_file.to_str().unwrap_or_default(),
                e
            ))
        })?;
        let lookup = RTree::bulk_load(data.to_vec());
        Ok(TimeDelayLookup {
            lookup,
            time_unit,
            agg,
        })
    }

    /// gets a delay value from this lookup function and returns it in the base time unit.
    /// when delays are not expected to overlap, this function only takes the first overlapping
    /// row and returns that value.
    ///
    /// # Arguments
    ///
    /// * `geometry` - geometry to find intersecting time access records
    ///
    /// # Returns
    ///
    /// * Zero or one time access delay. If addditional records intersect the incoming geometry,
    ///   only the first is returned.
    pub fn find_first_delay(&self, geometry: &Geometry<f32>) -> Option<(Time, &TimeUnit)> {
        let envelope_option: Option<AABB<Point<f32>>> =
            geo_utils::get_centroid_as_envelope(geometry);
        let result = envelope_option.and_then(|envelope| {
            let lookup_result = self
                .lookup
                .locate_in_envelope_intersecting(&envelope)
                .next();
            lookup_result
        });
        result.map(|t| (t.time, &self.time_unit))
    }

    /// gets a delay value from this lookup function and returns it in the base time unit.
    /// when many delays may overlap with this geometry, this function will take all intersecting
    /// rows and aggregate them together into a single delay value.
    ///
    /// # Arguments
    ///
    /// * `geometry` - geometry to find intersecting time access records
    ///
    /// # Returns
    ///
    /// * Zero or one time access delay. If addditional records intersect the incoming geometry,
    ///   only the first is returned.
    pub fn find_all_delays(&self, geometry: &Geometry<f32>) -> Option<(Time, &TimeUnit)> {
        let envelope_option: Option<AABB<Point<f32>>> =
            geo_utils::get_centroid_as_envelope(geometry);
        let time = envelope_option.and_then(|envelope| {
            let values = self
                .lookup
                .locate_in_envelope_intersecting(&envelope)
                .map(|record| record.time)
                .collect();
            self.agg.apply(values)
        });
        time.map(|t| (t, &self.time_unit))
    }
}

impl TryFrom<&serde_json::Value> for TimeDelayLookup {
    type Error = CompassConfigurationError;

    /// builds a new lookup function for zonal time delays at either trip departure or arrival
    ///
    /// # Arguments
    ///
    /// * `value` - JSON value for this lookup instance
    ///
    /// # Returns
    ///
    /// * an object that can be used to lookup time delay values, or an error
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let parent_key = String::from("time delay lookup");
        let lookup_file = value.get_config_path(&String::from("lookup_file"), &parent_key)?;
        let time_unit =
            value.get_config_serde::<TimeUnit>(&String::from("time_unit"), &parent_key)?;
        let agg = value
            .get_config_serde_optional::<DelayAggregationType>(&"aggregation", &parent_key)?
            .unwrap_or_default();
        let lookup = TimeDelayLookup::new(&lookup_file, time_unit, agg)?;
        Ok(lookup)
    }
}
