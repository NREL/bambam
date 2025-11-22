use jsonpath_rust::JsonPath;
use routee_compass::{
    app::{
        compass::CompassAppError,
        search::{SearchApp, SearchAppResult},
    },
    plugin::output::{OutputPlugin, OutputPluginError},
};
use routee_compass_core::algorithm::search::SearchInstance;
use serde_json::Value;
use std::sync::Arc;

pub struct H3UtilInputPlugin {}

impl OutputPlugin for H3UtilInputPlugin {
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        todo!()
    }
}
