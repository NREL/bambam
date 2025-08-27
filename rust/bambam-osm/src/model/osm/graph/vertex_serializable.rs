use routee_compass_core::model::network::{Vertex, VertexId};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct VertexSerializable {
    vertex_id: VertexId,
    x: f32,
    y: f32,
}

impl From<&Vertex> for VertexSerializable {
    fn from(value: &Vertex) -> Self {
        VertexSerializable {
            vertex_id: value.vertex_id,
            x: value.coordinate.x,
            y: value.coordinate.y,
        }
    }
}
