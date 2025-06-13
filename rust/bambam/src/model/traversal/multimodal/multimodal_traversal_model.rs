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
    pub output_features: Vec<(String, OutputFeature)>,
}

impl MultimodalTraversalModel {
    pub fn new(
        feature_dependencies: Vec<FeatureDependency>,
        output_features: Vec<(String, OutputFeature)>,
    ) -> MultimodalTraversalModel {
        MultimodalTraversalModel {
            feature_dependencies,
            output_features,
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
        self.output_features.to_vec()
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
