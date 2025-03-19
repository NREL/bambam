use serde::{Deserialize, Serialize};

/// An enumeration representing how activities are tagged to the graph.
#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityTableOrientation {
    OriginVertexOriented,
    DestinationVertexOriented,
    EdgeOriented,
}
