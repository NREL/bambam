mod adjacency_direction;
mod compass_writer;
pub mod fill_value_lookup;
pub mod osm_element_filter;
pub mod osm_graph;
mod osm_graph_vectorized;
pub mod osm_node_data;
mod osm_node_data_serializable;
mod osm_node_id;
pub mod osm_segment;
pub mod osm_way_data;
mod osm_way_data_serializable;
mod osm_way_id;
mod vertex_serializable;

use crate::model::osm::OsmError;
pub use adjacency_direction::AdjacencyDirection;
pub use compass_writer::CompassWriter;
use geo::{Coord, LineString};
use itertools::Itertools;
use kdam::tqdm;
use log;
use osm_element_filter::ElementFilter;
pub use osm_graph::OsmGraph;
pub use osm_graph_vectorized::OsmGraphVectorized;
pub use osm_node_data::OsmNodeData;
pub use osm_node_data_serializable::OsmNodeDataSerializable;
pub use osm_node_id::OsmNodeId;
use osm_segment::OsmSegment;
pub use osm_way_data::OsmWayData;
pub use osm_way_data_serializable::OsmWayDataSerializable;
pub use osm_way_id::OsmWayId;
use osmpbf::{Element, ElementReader};
use routee_compass_core::{
    model::{
        network::Vertex,
        unit::{Distance, DistanceUnit},
    },
    util::compact_ordered_hash_map::CompactOrderedHashMap,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
pub use vertex_serializable::VertexSerializable;

// type aliases for transitioning between OSM and internal (dense) representation
pub type Osmid = i64;
pub type CompassIndex = usize;

pub type OsmNodes = HashMap<OsmNodeId, OsmNodeData>;
pub type OsmWays = HashMap<OsmWayId, OsmWayData>;

pub type OsmNodesSerializable = Vec<OsmNodeDataSerializable>;
pub type OsmWaysSerializable = Vec<OsmWayDataSerializable>;
pub type VertexLookup = HashMap<OsmNodeId, (CompassIndex, Vertex)>;

pub type PathSegment = Vec<OsmNodeId>;
pub type AdjacencyListDeprecated = HashMap<OsmNodeId, HashMap<OsmNodeId, OsmSegment>>;
pub type AdjacencyList =
    HashMap<(OsmNodeId, AdjacencyDirection), HashMap<OsmNodeId, Vec<OsmSegment>>>;

pub type OsmWaysByOd = HashMap<(OsmNodeId, OsmNodeId), Vec<OsmWayData>>;
pub type AdjacencyList3 = HashMap<(OsmNodeId, AdjacencyDirection), HashSet<OsmNodeId>>;
pub type Path3 = Vec<OsmNodeId>;
