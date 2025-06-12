use crate::model::fieldname;

use super::FeatureDependency;
use itertools::Itertools;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{CustomFeatureFormat, InputFeature, OutputFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError},
};
use std::collections::HashMap;

pub struct MultimodalTraversalModel {
    pub feature_dependencies: Vec<FeatureDependency>,
}

impl MultimodalTraversalModel {
    pub fn new(feature_dependencies: Vec<FeatureDependency>) -> MultimodalTraversalModel {
        MultimodalTraversalModel {
            feature_dependencies,
        }
    }
}

impl TraversalModel for MultimodalTraversalModel {
    fn input_features(&self) -> Vec<(String, InputFeature)> {
        self.feature_dependencies
            .iter()
            .map(|f| f.as_input_feature())
            .collect_vec()
    }

    fn output_features(&self) -> Vec<(String, OutputFeature)> {
        vec![(
            fieldname::COST_PENALTY_FACTOR.to_string(),
            OutputFeature::Custom {
                name: fieldname::COST_PENALTY_FACTOR.to_string(),
                unit: String::from("coefficient"),
                format: CustomFeatureFormat::FloatingPoint {
                    initial: 0.0.into(),
                },
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
        for fd in self.feature_dependencies.iter() {
            fd.apply_feature_dependency(state, state_model)?;
        }
        Ok(())
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        for fd in self.feature_dependencies.iter() {
            fd.apply_feature_dependency(state, state_model)?;
        }
        Ok(())
    }
}
