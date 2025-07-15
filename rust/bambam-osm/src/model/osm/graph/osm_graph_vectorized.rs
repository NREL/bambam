use super::{
    CompassIndex, HashMap, Itertools, OsmError, OsmGraph, OsmNodeDataSerializable, OsmNodeId,
    OsmNodes, OsmNodesSerializable, OsmWayDataSerializable, OsmWaysSerializable, Vertex,
    VertexLookup,
};
use crate::model::osm::graph::{osm_segment::OsmSegment, AdjacencyDirection};
use kdam::tqdm;
use serde::{Deserialize, Serialize};

pub struct OsmGraphVectorized {
    /// the collection of OSM nodes associated via their OSMID
    pub nodes: OsmNodesSerializable,
    /// just a list of OSM ways in an arbitrary order. these are unique by OSMID but
    /// not guaranteed to be unique by source and destination node (i.e., multigraph).
    pub ways: OsmWaysSerializable,
    /// for each OsmNodeId, the vertex index
    pub vertex_lookup: VertexLookup,
    /// loaded and simplified/consolidated graph dataset
    pub reference_graph: OsmGraph,
}

impl OsmGraphVectorized {
    /// vectorizes an [`OsmGraph`] such that the position of each node and way in each vector
    /// (their index) becomes their respective VectorId/EdgeId.
    pub fn new(graph: OsmGraph) -> Result<OsmGraphVectorized, OsmError> {
        // create vertex_ids, serializable nodes and vertex lookup (one-pass)
        let mut nodes: OsmNodesSerializable = Vec::with_capacity(graph.n_connected_nodes());
        let mut vertex_lookup: HashMap<OsmNodeId, (CompassIndex, Vertex)> = HashMap::new();
        let node_iter = tqdm!(
            graph.connected_node_data_iterator(true).enumerate(),
            total = graph.n_connected_nodes(),
            desc = "osm nodes to compass vertices"
        );
        for (vertex_id, result) in node_iter {
            let node = result?;
            let node_ser = OsmNodeDataSerializable::from(node);
            nodes.insert(vertex_id, node_ser);

            let vertex = Vertex::new(vertex_id, node.x, node.y);
            vertex_lookup.insert(node.osmid, (vertex_id, vertex));
        }
        eprintln!();

        // create edge_ids and serializable ways
        let edge_iter = tqdm!(
            graph.connected_ways_triplet_iterator(true),
            total = graph.n_connected_ways(),
            desc = "osm ways to compass edges"
        );
        let mut ways: OsmWaysSerializable = vec![];
        for (idx, traj_result) in edge_iter.enumerate() {
            match traj_result {
                Ok(None) => {}
                Ok(Some(traj)) if traj.is_empty() => {
                    return Err(OsmError::InternalError(format!(
                        "way with EdgeId {} has no trajectories",
                        idx
                    )))
                }
                Ok(Some(traj)) => {
                    let result = OsmWayDataSerializable::new(traj, &graph, &vertex_lookup)?;
                    ways.push(result);
                }
                Err(e) => return Err(OsmError::GraphModificationError(e.to_string())),
            }
        }
        eprintln!();

        let result = OsmGraphVectorized {
            nodes,
            ways,
            vertex_lookup,
            reference_graph: graph,
        };
        Ok(result)
    }
}
