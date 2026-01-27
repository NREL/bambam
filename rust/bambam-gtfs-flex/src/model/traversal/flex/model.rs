use std::sync::Arc;

use chrono::NaiveDateTime;
use routee_compass_core::{
    algorithm::search::SearchTree,
    model::{
        network::{Edge, Vertex},
        state::{InputFeature, StateModel, StateVariable, StateVariableConfig},
        traversal::{TraversalModel, TraversalModelError},
    },
};
use uom::{si::f64::Time, ConstZero};

use crate::model::{feature, traversal::flex::GtfsFlexTraversalEngine};

pub struct GtfsFlexTraversalModel {
    engine: Arc<GtfsFlexTraversalEngine>,
    start_time: Option<NaiveDateTime>,
}

impl GtfsFlexTraversalModel {
    pub fn new(engine: Arc<GtfsFlexTraversalEngine>, start_time: Option<NaiveDateTime>) -> Self {
        Self { engine, start_time }
    }
}

impl TraversalModel for GtfsFlexTraversalModel {
    fn name(&self) -> String {
        "GtfsFlexTraversalModel".to_string()
    }

    fn input_features(&self) -> Vec<InputFeature> {
        vec![]
    }

    fn output_features(&self) -> Vec<(String, StateVariableConfig)> {
        let mut base_features = vec![
            (
                feature::fieldname::TRIP_SRC_ZONE_ID.to_string(),
                feature::variable::zone_id(),
            ),
            (
                feature::fieldname::EDGE_IS_GTFS_FLEX_DESTINATION.to_string(),
                feature::variable::gtfs_flex_destination(),
            ),
            (
                String::from(routee_compass_core::model::traversal::default::fieldname::TRIP_TIME),
                StateVariableConfig::Time {
                    initial: Time::ZERO,
                    output_unit: None,
                    accumulator: true,
                },
            ),
            (
                String::from(routee_compass_core::model::traversal::default::fieldname::EDGE_TIME),
                StateVariableConfig::Time {
                    initial: Time::ZERO,
                    output_unit: None,
                    accumulator: false,
                },
            ),
            // (
            //     String::from(bambam_state::ROUTE_ID),
            //     StateVariableConfig::Custom {
            //         custom_type: "RouteId".to_string(),
            //         value: EMPTY,
            //         accumulator: true,
            //     },
            // ),
            // (
            //     String::from(bambam_state::TRANSIT_BOARDING_TIME),
            //     StateVariableConfig::Time {
            //         initial: Time::ZERO,
            //         accumulator: false,
            //         output_unit: None,
            //     },
            // ),
        ];
        if false {
            base_features.push((
                feature::fieldname::EDGE_POOLING_DELAY.to_string(),
                feature::variable::pooling_delay(),
            ));
        }
        base_features
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        _tree: &routee_compass_core::algorithm::search::SearchTree,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let (_, edge, _) = trajectory;
        self.engine
            .traverse_edge(edge, state, state_model, self.start_time.as_ref())
    }

    fn estimate_traversal(
        &self,
        _od: (&Vertex, &Vertex),
        _state: &mut Vec<StateVariable>,
        _tree: &SearchTree,
        _state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        // no estimates
        Ok(())
    }
}
