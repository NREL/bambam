use crate::model::fieldname;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{InputFeature, OutputFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError, TraversalModelService},
    unit::{Speed, SpeedUnit},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FixedSpeedModel {
    pub speed: Speed,
    pub speed_unit: SpeedUnit,
}

impl TraversalModelService for FixedSpeedModel {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let model: Arc<dyn TraversalModel> = Arc::new(self.clone());
        Ok(model)
    }
}

impl TraversalModel for FixedSpeedModel {
    fn input_features(&self) -> Vec<(String, InputFeature)> {
        vec![]
    }

    fn output_features(&self) -> Vec<(String, OutputFeature)> {
        vec![(
            fieldname::EDGE_SPEED.to_string(),
            OutputFeature::Speed {
                speed_unit: self.speed_unit,
                initial: Speed::ZERO,
                accumulator: false,
            },
        )]
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        state_model.set_speed(state, fieldname::EDGE_SPEED, &self.speed, &self.speed_unit)?;
        Ok(())
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        state_model.set_speed(state, fieldname::EDGE_SPEED, &self.speed, &self.speed_unit)?;
        Ok(())
    }
}
