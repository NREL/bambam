use std::sync::Arc;

use chrono::NaiveDateTime;
use routee_compass_core::model::traversal::TraversalModel;

use crate::model::traversal::flex::{GtfsFlexModelState, GtfsFlexTraversalEngine};

pub struct GtfsFlexTraversalModel {
    model_state: GtfsFlexModelState,
    start_time: Option<NaiveDateTime>,
}

impl GtfsFlexTraversalModel {
    pub fn new(engine: Arc<GtfsFlexTraversalEngine>, start_time: Option<NaiveDateTime>) -> Self {
        use GtfsFlexTraversalEngine as E;
        match engine.clone().as_ref() {
            E::ServiceTypeFour { gtfs } => {
                let delays = vec![].into_boxed_slice();
                let model_state = GtfsFlexModelState::TypeFourWithDelays {
                    gtfs: gtfs.clone(),
                    delays,
                };
                Self {
                    model_state,
                    start_time,
                }
            }
            _ => Self {
                model_state: GtfsFlexModelState::EngineOnly(engine.clone()),
                start_time,
            },
        }
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
        _trajectory: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Edge,
            &routee_compass_core::model::network::Vertex,
        ),
        _state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        _tree: &routee_compass_core::algorithm::search::SearchTree,
        _state_model: &routee_compass_core::model::state::StateModel,
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
        _od: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Vertex,
        ),
        _state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        _tree: &routee_compass_core::algorithm::search::SearchTree,
        _state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::traversal::TraversalModelError> {
        // no estimates
        Ok(())
    }
}
