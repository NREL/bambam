//! values deserialized from a search query which can be used to override defaults.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalAccessQuery {
    /// allows, at query time, for users to modify the list of available modes for a search.
    /// if not provided, the [`super::MultimodalTraversalConfig`] value will be used.
    pub available_modes: Option<Vec<String>>,
}
