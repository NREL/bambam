use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};

use crate::model::traversal::gbfs::GbfsTraversalConfig;

pub struct GbfsTraversalService {
    pub config: GbfsTraversalConfig,
}

impl GbfsTraversalService {
    pub fn new(config: GbfsTraversalConfig) -> GbfsTraversalService {
        GbfsTraversalService { config }
    }
}

impl TraversalModelService for GbfsTraversalService {
    fn build(
        &self,
        _query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        // if there's anything that can change between the execution of each search,
        // we should attempt to pull it from the query here.
        todo!()
    }
}
