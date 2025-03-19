use super::{super::super::bambam_state_ops, schedule_traversal_engine::ScheduleTraversalEngine};
use chrono::{DateTime, Utc};
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{CustomFeatureFormat, StateFeature, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use std::sync::Arc;

/// Traversal Model for a fixed-speed mode
pub struct ScheduleTraversalModel {
    pub engine: Arc<ScheduleTraversalEngine>,
    pub start_time: DateTime<Utc>,
}

impl TraversalModel for ScheduleTraversalModel {
    fn state_features(&self) -> Vec<(String, StateFeature)> {
        let mut features = bambam_state_ops::default_state_features();
        features.push((
            String::from(bambam_state_ops::field::ROUTE_ID),
            StateFeature::Custom {
                r#type: String::from("route id"),
                unit: String::from("signed integer"),
                format: CustomFeatureFormat::SignedInteger {
                    initial: bambam_state_ops::field::EMPTY_ROUTE_ID,
                },
            },
        ));
        features
    }

    fn traverse_edge(
        &self,
        _trajectory: (&Vertex, &Edge, &Vertex),
        _state: &mut Vec<StateVariable>,
        _state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }

    fn estimate_traversal(
        &self,
        _od: (&Vertex, &Vertex),
        _state: &mut Vec<StateVariable>,
        _state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), TraversalModelError> {
        todo!()
    }
}
