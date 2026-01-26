use std::sync::Arc;

use chrono::NaiveDateTime;
use routee_compass_core::model::traversal::{TraversalModelError, TraversalModelService};

use crate::model::traversal::flex::{
    GtfsFlexServiceTypeTwoQuery, GtfsFlexTraversalEngine, GtfsFlexTraversalModel,
};

pub struct GtfsFlexTraversalService {
    engine: Arc<GtfsFlexTraversalEngine>,
}

impl TraversalModelService for GtfsFlexTraversalService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<
        std::sync::Arc<dyn routee_compass_core::model::traversal::TraversalModel>,
        routee_compass_core::model::traversal::TraversalModelError,
    > {
        // if this is a type two query, we grab the start datetime
        // todo: also should apply in type 3
        let start_time: Option<NaiveDateTime> = if self.engine.requires_start_time() {
            let query: GtfsFlexServiceTypeTwoQuery = serde_json::from_value(query.clone())
                .map_err(|e| {
                    TraversalModelError::BuildError(format!(
                        "failure reading service type two query: {e}"
                    ))
                })?;
            Some(query.start_time)
        } else {
            None
        };
        Ok(Arc::new(GtfsFlexTraversalModel::new(
            self.engine.clone(),
            start_time,
        )))
    }
}

impl GtfsFlexTraversalService {
    pub fn new(engine: GtfsFlexTraversalEngine) -> Self {
        Self {
            engine: Arc::new(engine),
        }
    }
}
