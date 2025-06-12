mod dependency_unit_type;
mod feature_dependency;
mod multimodal_traversal_builder;
mod multimodal_traversal_config;
mod multimodal_traversal_model;
mod multimodal_traversal_service;

pub use dependency_unit_type::DependencyUnitType;
pub use feature_dependency::FeatureDependency;
pub use multimodal_traversal_builder::MultimodalTraversalBuilder;
pub use multimodal_traversal_config::MultimodalTraversalConfig;
pub use multimodal_traversal_model::MultimodalTraversalModel;

use std::collections::HashMap;

/// alias for a mapping from mode name to it's set of feature dependencies.
/// each feature dependency contains the information for how it influences mode-
/// specific state updates.
pub type FeatureMappingsByMode = HashMap<String, Vec<FeatureDependency>>;
