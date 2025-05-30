use super::FixedSpeedModeConfig;
use crate::model::traversal::fixed_speed::FixedSpeedBuilder;
use routee_compass_core::model::traversal::{
    default::{
        combined::{CombinedTraversalBuilder, CombinedTraversalService},
        distance::DistanceTraversalBuilder,
        time::TimeTraversalBuilder,
    },
    TraversalModelBuilder, TraversalModelError, TraversalModelService,
};
use std::sync::Arc;

/// convenience traversal modeling implementation for fixed traversal modes, such as bike or walk,
/// where the same speed is used for each road network link.
///
/// # Example
///
/// Configuration of a fixed-mode traversal:
///
/// ```toml
/// [traversal]
/// type = "fixed"
/// name = "walk"
/// distance_unit = "miles"
/// time_unit = "minutes"
/// speed_unit = "kph"
/// speed = 5.0
///
/// [traversal]
/// type = "fixed"
/// name = "bike"
/// distance_unit = "miles"
/// time_unit = "minutes"
/// speed_unit = "kph"
/// speed = 16.0
/// ```
pub struct FixedSpeedModeBuilder {}

impl TraversalModelBuilder for FixedSpeedModeBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModelService>, TraversalModelError> {
        let config = FixedSpeedModeConfig::try_from(parameters)?;
        let service = config.build();
        Ok(Arc::new(service))
    }
}
