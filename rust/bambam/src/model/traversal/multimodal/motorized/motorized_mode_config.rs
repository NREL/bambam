use routee_compass_core::model::{
    traversal::{
        default::{
            combined::CombinedTraversalService,
            distance::{DistanceTraversalBuilder, DistanceTraversalService},
            time::{TimeTraversalBuilder, TimeTraversalModel},
        },
        TraversalModelBuilder, TraversalModelError, TraversalModelService,
    },
    unit::{DistanceUnit, Speed, SpeedUnit, TimeUnit},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MotorizedModeConfig {
    pub distance_unit: Option<DistanceUnit>,
    pub time_unit: Option<TimeUnit>,
    pub speed: Speed,
    pub speed_unit: SpeedUnit,
    pub weights_input_file: Option<String>,
}

impl TryFrom<&serde_json::Value> for MotorizedModeConfig {
    type Error = TraversalModelError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value.clone()).map_err(|e| {
            TraversalModelError::BuildError(format!(
                "failed to build fixed mode traversal model: {}",
                e
            ))
        })
    }
}

impl MotorizedModeConfig {
    pub fn build(&self) -> CombinedTraversalService {
        let distance_unit = self.distance_unit.unwrap_or(DistanceUnit::Miles);
        let time_unit = self.time_unit.unwrap_or(TimeUnit::Minutes);

        let d = Arc::new(DistanceTraversalService { distance_unit });
        let t = Arc::new(TimeTraversalModel::new(&time_unit));
        // OH! to stay compartmentalized, we write a different Time state model that switches how it grabs
        // speed values based on the travel mode
        let s: Arc<dyn TraversalModelService> = todo!();
        let services: Vec<Arc<dyn TraversalModelService>> = vec![d, s, t];

        CombinedTraversalService::new(services)
    }
}
