use serde::{Deserialize, Serialize};

/// configuration for the multimodal labeling model. this type is deserialized from the
/// config [label] section and set as defaults for the label model. at query time, this
/// can be deserialized again to override defaults.
///
/// all values must be _optional_ as an invariant for the deserialization algorithm
/// used by the [`MultimodalLabelService`].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultimodalLabelConfig {
    pub max_trip_legs: Option<usize>,
    pub modes: Option<Vec<String>>,
}
