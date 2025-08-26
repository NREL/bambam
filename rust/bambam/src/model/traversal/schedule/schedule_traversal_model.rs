use super::{super::super::bambam_state_ops, schedule_traversal_engine::ScheduleTraversalEngine};
use crate::model::fieldname;
use chrono::{DateTime, Utc};
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{CustomVariableConfig, InputFeature, StateVariableConfig, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use std::sync::Arc;

/// Traversal Model for a fixed-speed mode
pub struct ScheduleTraversalModel {
    pub engine: Arc<ScheduleTraversalEngine>,
    pub start_time: DateTime<Utc>,
}

impl ScheduleTraversalModel {
    const EMPTY_ROUTE_ID: i64 = -1;
}

impl TraversalModel for ScheduleTraversalModel {
    fn name(&self) -> String {
        "Schedule Traversal Model".to_string()
    }

    fn input_features(&self) -> Vec<InputFeature> {
        vec![]
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        vec![(
            String::from(fieldname::ROUTE_ID),
            StateVariableConfig::Custom {
                custom_type: String::from("RouteId"),
                value: CustomVariableConfig::SignedInteger {
                    initial: Self::EMPTY_ROUTE_ID,
                },
                accumulator: false,
            },
        )]
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
