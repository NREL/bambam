use chrono::{DateTime, Utc};
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::traversal::TraversalModel;
use routee_compass_core::model::traversal::TraversalModelError;
use routee_compass_core::model::traversal::TraversalModelService;
use std::sync::Arc;

use super::schedule_traversal_engine::ScheduleTraversalEngine;
use super::schedule_traversal_model::ScheduleTraversalModel;

pub struct ScheduleTraversalService {
    pub engine: Arc<ScheduleTraversalEngine>,
}

impl ScheduleTraversalService {
    pub fn new(
        params: &serde_json::Value,
    ) -> Result<ScheduleTraversalService, TraversalModelError> {
        let engine = ScheduleTraversalEngine::new(params)?;
        let result = ScheduleTraversalService {
            engine: Arc::new(engine),
        };
        Ok(result)
    }
}

impl TraversalModelService for ScheduleTraversalService {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let start_time: DateTime<Utc> = query
            .get_config_serde(&"start_time", &"schedule traversal model")
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        let engine = self.engine.clone();
        let model = ScheduleTraversalModel { engine, start_time };
        Ok(Arc::new(model))
    }
}
