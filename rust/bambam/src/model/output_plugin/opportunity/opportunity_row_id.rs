use std::sync::Arc;

use geo::{Centroid, Convert, LineString};
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::{
    algorithm::search::{SearchInstance, SearchTreeBranch},
    model::{
        map::MapModel,
        network::{EdgeId, Graph, VertexId},
    },
};
use rstar::{RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use wkt::ToWkt;

use crate::model::output_plugin::opportunity::{
    opportunity_format::OpportunityFormat, opportunity_orientation::OpportunityOrientation,
};

// identifier in the graph tagging where an opportunity was found
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum OpportunityRowId {
    OriginVertex(VertexId),
    DestinationVertex(VertexId),
    Edge(EdgeId),
}

impl std::fmt::Display for OpportunityRowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_usize())
    }
}

impl OpportunityRowId {
    /// create a new opportunity vector identifier based on the table orientation which denotes where opportunities are stored
    pub fn new(
        branch_vertex_id: &VertexId,
        branch: &SearchTreeBranch,
        format: &OpportunityOrientation,
    ) -> OpportunityRowId {
        use OpportunityOrientation as O;
        match format {
            // stored at the origin of the edge, corresponding with the branch origin id
            O::OriginVertexOriented => Self::OriginVertex(*branch_vertex_id),
            // stored at the destination of the edge at the branch's terminal vertex id
            O::DestinationVertexOriented => Self::DestinationVertex(branch.terminal_vertex),
            // stored on the edge itself
            O::EdgeOriented => Self::Edge(branch.edge_traversal.edge_id),
        }
    }

    /// helper to get the underlying usize value from this index
    pub fn as_usize(&self) -> &usize {
        match self {
            OpportunityRowId::OriginVertex(v) => &v.0,
            OpportunityRowId::DestinationVertex(v) => &v.0,
            OpportunityRowId::Edge(e) => &e.0,
        }
    }

    /// helper to get the POINT geometry associated with this index, if defined
    pub fn get_vertex_point(
        &self,
        graph: Arc<Graph>,
    ) -> Result<geo::Point<f32>, OutputPluginError> {
        let vertex_id = match self {
            OpportunityRowId::OriginVertex(vertex_id) => Ok(vertex_id),
            OpportunityRowId::DestinationVertex(vertex_id) => Ok(vertex_id),
            OpportunityRowId::Edge(edge_id) => Err(OutputPluginError::InternalError(String::from(
                "cannot get vertex point for edge",
            ))),
        }?;

        let vertex = graph.get_vertex(vertex_id).map_err(|e| {
            OutputPluginError::OutputPluginFailed(format!("unknown vertex id '{}'", vertex_id))
        })?;
        let point = geo::Point::new(vertex.x(), vertex.y());
        Ok(point)
    }

    /// helper to get the LINESTRING geometry associated with this index, if defined
    pub fn get_edge_linestring(
        &self,
        map_model: Arc<MapModel>,
    ) -> Result<geo::LineString<f32>, OutputPluginError> {
        let edge_id = match self {
            OpportunityRowId::Edge(edge_id) => Ok(edge_id),
            _ => Err(OutputPluginError::InternalError(String::from(
                "cannot get edge linestring for vertex",
            ))),
        }?;
        map_model.get(edge_id).cloned().map_err(|e| {
            OutputPluginError::OutputPluginFailed(format!("unknown edge id '{}'", edge_id))
        })
    }

    pub fn get_envelope_f64(
        &self,
        si: &SearchInstance,
    ) -> Result<AABB<geo::Point>, OutputPluginError> {
        match self {
            OpportunityRowId::OriginVertex(_) => {
                let point = self.get_vertex_point(si.graph.clone())?.convert();
                Ok(point.envelope())
            }
            OpportunityRowId::DestinationVertex(_) => {
                let point = self.get_vertex_point(si.graph.clone())?.convert();
                Ok(point.envelope())
            }
            OpportunityRowId::Edge(_) => {
                let linestring = self.get_edge_linestring(si.map_model.clone())?.convert();
                Ok(linestring.envelope())
            }
        }
    }

    pub fn get_centroid_f64(&self, si: &SearchInstance) -> Result<geo::Point, OutputPluginError> {
        match self {
            OpportunityRowId::OriginVertex(_) => {
                let point = self.get_vertex_point(si.graph.clone())?.convert();
                let centroid = point.centroid();
                Ok(centroid)
            }
            OpportunityRowId::DestinationVertex(_) => {
                let point = self.get_vertex_point(si.graph.clone())?.convert();
                let centroid = point.centroid();
                Ok(centroid)
            }
            OpportunityRowId::Edge(_) => {
                let linestring = self.get_edge_linestring(si.map_model.clone())?.convert();
                let centroid = linestring.centroid().ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "could not get centroid of LINESTRING {}",
                        linestring.to_wkt()
                    ))
                })?;
                Ok(centroid)
            }
        }
    }
}
