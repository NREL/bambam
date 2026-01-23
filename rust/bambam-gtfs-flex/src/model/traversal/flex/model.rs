use std::sync::Arc;

use chrono::NaiveDateTime;
use routee_compass_core::model::traversal::TraversalModel;

use crate::model::traversal::flex::GtfsFlexTraversalEngine;

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

    fn input_features(&self) -> Vec<routee_compass_core::model::state::InputFeature> {
        todo!()
    }

    fn output_features(
        &self,
    ) -> Vec<(
        String,
        routee_compass_core::model::state::StateVariableConfig,
    )> {
        todo!()
    }

    fn traverse_edge(
        &self,
        trajectory: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Edge,
            &routee_compass_core::model::network::Vertex,
        ),
        state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        tree: &routee_compass_core::algorithm::search::SearchTree,
        state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::traversal::TraversalModelError> {
        todo!("
            1. grab the Option<ZoneId> crate::model::feature::fieldname::SRC_ZONE_ID from the state (using state_model)
            2. get the Option<ZoneId> of this edge (todo: rescue the multimodal mapping tool from bambam here)
                - if it is None, we are done
            3. if zone ids match, this is a valid destination -> set crate::model::feature::fieldname::IS_DESTINATION
        ")
    }

    fn estimate_traversal(
        &self,
        od: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Vertex,
        ),
        state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        tree: &routee_compass_core::algorithm::search::SearchTree,
        state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::traversal::TraversalModelError> {
        // no estimates
        Ok(())
    }
}
