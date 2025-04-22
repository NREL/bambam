use super::{
    osm_graph::OsmGraph, osm_way_data::OsmWayData,
    osm_way_data_serializable::OsmWayDataSerializable,
};
use crate::model::osm::OsmError;
use kdam::tqdm;
use std::collections::HashMap;

/// looks up average values by class labels in order to fill incomplete data taken
/// from OpenStreetMaps, such as maxspeed entries.
pub struct FillValueLookup {
    pub class_field: String,
    pub value_field: String,
    pub values_by_class: HashMap<String, f64>,
    pub global_average: f64,
}

impl FillValueLookup {
    pub fn new<'a>(
        ways: &[OsmWayDataSerializable],
        class_label_field: &str,
        value_field: &str,
        value_op: impl Fn(&OsmWayDataSerializable) -> Result<Option<f64>, OsmError>,
    ) -> Result<FillValueLookup, OsmError> {
        let mut buckets: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();
        let way_iter = tqdm!(
            ways.iter(),
            desc = "collect fill values",
            total = ways.len()
        );
        for way in way_iter {
            let length_meters = way.length_meters as f64;
            let class_label_opt = way
                .get_string_at_field(class_label_field)
                .map_err(OsmError::GraphConsolidationError)?;
            let value_opt = value_op(way)?;
            if let (Some(class_label), Some(value)) = (class_label_opt, value_opt) {
                let _ = buckets
                    .entry(class_label)
                    .and_modify(|(vs, ds)| {
                        vs.push(value);
                        ds.push(length_meters);
                    })
                    .or_default();
            }
        }

        let mut global_numer: f64 = 0.0;
        let mut global_denom: f64 = 0.0;
        let bucket_iter = tqdm!(
            buckets.iter(),
            desc = "aggregate fill values",
            total = buckets.len()
        );
        let values_by_class = bucket_iter
            .map(|(k, (vs, ds))| {
                let numer: f64 = vs.iter().zip(ds).map(|(v, d)| v * d).sum();
                let denom: f64 = ds.iter().sum();
                global_numer += numer;
                global_denom += denom;
                let agg_value = numer / denom;
                (k.clone(), agg_value)
            })
            .collect::<HashMap<String, f64>>();

        let global_average = global_numer / global_denom;
        let result = FillValueLookup {
            class_field: String::from(class_label_field),
            value_field: String::from(value_field),
            values_by_class,
            global_average,
        };

        Ok(result)
    }

    /// get a fill value. if label is provided, we try and see if the value exists.
    /// if no label is provided or no fill value is found, then we simply return the global average.
    pub fn get(&self, label: &Option<String>) -> f64 {
        match label {
            Some(l) => *self.values_by_class.get(l).unwrap_or(&self.global_average),
            None => self.global_average,
        }
    }
}
