use routee_compass_core::model::access::AccessModel;

pub struct MultimodalAccessModel {}

impl AccessModel for MultimodalAccessModel {
    fn state_features(
        &self,
    ) -> Vec<(
        String,
        routee_compass_core::model::state::StateVariableConfig,
    )> {
        todo!()
    }

    fn access_edge(
        &self,
        traversal: (
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Edge,
            &routee_compass_core::model::network::Vertex,
            &routee_compass_core::model::network::Edge,
            &routee_compass_core::model::network::Vertex,
        ),
        state: &mut Vec<routee_compass_core::model::state::StateVariable>,
        state_model: &routee_compass_core::model::state::StateModel,
    ) -> Result<(), routee_compass_core::model::access::AccessModelError> {
        todo!()
    }
}
