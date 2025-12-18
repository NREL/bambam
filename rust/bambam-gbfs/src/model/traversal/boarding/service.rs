use std::sync::Arc;

use routee_compass_core::model::traversal::{
    TraversalModel, TraversalModelError, TraversalModelService,
};

use super::BoardingTraversalConfig;

pub struct BoardingTraversalService {
    pub config: BoardingTraversalConfig,
}

impl BoardingTraversalService {
    pub fn new(config: BoardingTraversalConfig) -> BoardingTraversalService {
        BoardingTraversalService { config }
    }
}

impl TraversalModelService for BoardingTraversalService {
    fn build(
        &self,
        _query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        // if there's anything that can change between the execution of each search,
        // we should attempt to pull it from the query here.
        todo!()
    }
}
