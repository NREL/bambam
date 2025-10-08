use routee_compass_core::model::{
    state::StateVariableConfig,
    traversal::{default::fieldname, TraversalModel},
};
use uom::{si::f64::Time, ConstZero};

pub struct TransitTraversalModel {}

impl TraversalModel for TransitTraversalModel {
    fn name(&self) -> String {
        todo!()
    }

    fn input_features(&self) -> Vec<routee_compass_core::model::state::InputFeature> {
        vec![
        ]
    }

    fn output_features(
        &self,
    ) -> Vec<(
        String,
        routee_compass_core::model::state::StateVariableConfig,
    )> {
        vec![
            (
                String::from(fieldname::TRIP_TIME),
                StateVariableConfig::Time {
                    initial: Time::ZERO,
                    output_unit: None, // TODO: Do I need to include a unit in the config?
                    accumulator: true,
                },
            ),
            // EDGE_TIME

            // (
            // String::from(fieldname::ROUTE_ID),
            // StateVariableConfig::Custom {
            //     custom_type: String::from("RouteId"),
            //     value: CustomVariableConfig::SignedInteger {
            //         initial: Self::EMPTY_ROUTE_ID,
            //     },
            //     accumulator: false,
            // })
        ]
    }

    fn traverse_edge(
        &self,
        trajectory: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Edge,
            &routee_compass_core::model::network::Vertex,
        ),
        state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::traversal::TraversalModelError> {
        todo!()
    }

    fn estimate_traversal(
        &self,
        od: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Vertex,
        ),
        state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::traversal::TraversalModelError> {
        todo!()
    }
}
