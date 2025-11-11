use super::finalize_ops;
use routee_compass::{
    app::{compass::CompassAppError, search::SearchAppResult},
    plugin::output::{OutputPlugin, OutputPluginError},
};
use routee_compass_core::algorithm::search::SearchInstance;
pub struct FinalizeOutputPlugin {
    pub id_field_name: String,
}

impl FinalizeOutputPlugin {
    /// a collection of tuples, where the first element is our target key,
    /// and the second is the path to the source value in the JSON output object.
    /// these are all assumed to be optional fields in the event of failure.
    pub const FIELD_NAMES_AND_PATHS: &'static [(&'static str, &'static [&'static str]); 12] = &[
        ("origin_edge", &["request", "origin_edge"]),
        ("mode", &["request", "mode"]),
        ("origin_x", &["request", "origin_x"]),
        ("origin_y", &["request", "origin_y"]),
        ("time", &["time_bin", "max_time"]),
        ("isochrone", &["isochrone"]),
        ("population", &["request", "population"]),
        ("tree_edge_count", &["tree_edge_count"]),
        ("search_runtime", &["search_runtime"]),
        ("basic_summary_runtime", &["basic_summary_runtime"]),
        ("result_memory_usage_bytes", &["result_memory_usage_bytes"]),
        ("error", &["error"]),
    ];

    // these keys have map structures in them that we can flatten. each pair
    // has the source key and the prefix to use when flattening/normalizing.
    // these are assumed to never fail.
    pub const FLATTEN_NESTED: &'static [(&'static str, &'static str); 2] =
        &[("opportunities", "opps"), ("mep", "mep")];
}

impl OutputPlugin for FinalizeOutputPlugin {
    /// reduces the output JSON to a subset of the fields that would appear in a normal
    /// RouteE Compass response. this produces a mostly flat response object.
    fn process(
        &self,
        output: &mut serde_json::Value,
        _: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        println!(
            "beginning finalize with output:\n {}",
            serde_json::to_string_pretty(output).unwrap()
        );

        // because we still annotate all response objects with keys
        let mut finalized = serde_json::Map::new();

        // grab the original request id
        let id = finalize_ops::get_value(output, &["request", &self.id_field_name])?;
        finalized.insert(self.id_field_name.clone(), id.clone());

        // attach all other expected keys
        for (name, path) in FinalizeOutputPlugin::FIELD_NAMES_AND_PATHS.iter() {
            let _insert_result = match finalize_ops::get_optional_value(output, path) {
                Some(value) => finalized.insert(name.to_string(), value.clone()),
                None => None,
            };
        }
        for (name, prefix) in FinalizeOutputPlugin::FLATTEN_NESTED.iter() {
            let map_kvs = finalize_ops::get_map_kvs(output, name)?;
            for (k, v) in map_kvs.iter() {
                let new_k = format!("{prefix}_{k}");
                finalized.insert(new_k, (*v).clone());
            }
        }

        let mut finalized_json = serde_json::json!(finalized);
        std::mem::swap(output, &mut finalized_json);
        Ok(())
    }
}
