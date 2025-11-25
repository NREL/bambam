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

use crate::model::output_plugin::h3_util::{DotDelimitedPath, H3Util, H3UtilOutputPluginConfig};

pub struct H3UtilInputPlugin {
    util: H3Util,
}

impl H3UtilInputPlugin {
    pub fn new(util: H3Util) -> H3UtilInputPlugin {
        H3UtilInputPlugin { util }
    }
}

impl OutputPlugin for H3UtilInputPlugin {
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        self.util.apply(output)
    }
}
