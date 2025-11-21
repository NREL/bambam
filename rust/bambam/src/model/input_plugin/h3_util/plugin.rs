use std::sync::Arc;

use routee_compass::{
    app::search::SearchApp,
    plugin::input::{InputPlugin, InputPluginError},
};
use serde_json::Value;

pub struct H3UtilInputPlugin {}

impl InputPlugin for H3UtilInputPlugin {
    fn process(
        &self,
        input: &mut Value,
        search_app: Arc<SearchApp>,
    ) -> Result<(), InputPluginError> {
        todo!()
    }
}
