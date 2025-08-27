use crate::model::osm::graph::OsmWayId;
use geo::Geometry;
use routee_compass_core::model::network::VertexId;

#[allow(unused)]
pub struct MinimalWay {
    pub osmid: OsmWayId,
    pub src_vertex_id: VertexId,
    pub dst_vertex_id: VertexId,
    pub geometry: Geometry,
    pub sidewalk: Option<String>,
    pub highway: Option<String>,
}
