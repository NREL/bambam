use crate::model::traversal::fixed_speed::FixedSpeedConfig;
use chrono::format::Fixed;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{InputFeature, OutputFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError, TraversalModelService},
    unit::{Speed, SpeedUnit},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct FixedSpeedModel {
    /// configuration of this model
    pub config: Arc<FixedSpeedConfig>,
    /// name of state feature where these speed values are assigned
    pub fieldname: String,
}

impl FixedSpeedModel {
    pub fn new(config: Arc<FixedSpeedConfig>) -> FixedSpeedModel {
        let fieldname = format!("{}_speed", config.name);
        FixedSpeedModel { config, fieldname }
    }
}

impl TraversalModelService for FixedSpeedModel {
    fn build(
        &self,
        _query: &serde_json::Value,
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
            self.fieldname.clone(),
            OutputFeature::Speed {
                speed_unit: self.config.speed_unit,
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
        state_model.set_speed(
            state,
            &self.fieldname,
            &self.config.speed,
            &self.config.speed_unit,
        )?;
        Ok(())
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        state_model.set_speed(
            state,
            &self.fieldname,
            &self.config.speed,
            &self.config.speed_unit,
        )?;
        Ok(())
    }
}
