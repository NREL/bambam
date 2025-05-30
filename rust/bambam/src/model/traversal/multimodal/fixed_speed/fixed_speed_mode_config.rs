use crate::model::traversal::fixed_speed::{FixedSpeedBuilder, FixedSpeedModel};
use routee_compass_core::model::{
    traversal::{
        default::{
            combined::CombinedTraversalService,
            distance::{DistanceTraversalBuilder, DistanceTraversalService},
            time::TimeTraversalBuilder,
        },
        TraversalModelBuilder, TraversalModelError, TraversalModelService,
    },
    unit::{DistanceUnit, Speed, SpeedUnit, TimeUnit},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedSpeedModeConfig {
    pub distance_unit: Option<DistanceUnit>,
    pub time_unit: Option<TimeUnit>,
    pub speed: Speed,
    pub speed_unit: SpeedUnit,
    pub weights_input_file: Option<String>,
}

impl TryFrom<&serde_json::Value> for FixedSpeedModeConfig {
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

impl FixedSpeedModeConfig {
    pub fn build(&self) -> CombinedTraversalService {
        let d = Arc::new(DistanceTraversalService {
            distance_unit: self.distance_unit.unwrap_or(DistanceUnit::Miles),
        });
        let s = Arc::new(FixedSpeedModel {
            speed: self.speed,
            speed_unit: self.speed_unit,
        });
        // should be fixed Wednesday
        let t = TimeTraversalBuilder {}
            .build(&json!({
                "time_unit": self.time_unit.unwrap_or(TimeUnit::Minutes)
            }))
            .expect("");
        let services: Vec<Arc<dyn TraversalModelService>> = vec![d, s, t];

        
        CombinedTraversalService::new(services)
    }
}
