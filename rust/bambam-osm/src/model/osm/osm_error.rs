use thiserror::Error;

use super::graph::{AdjacencyDirection, OsmNodeId, OsmWayId};

#[derive(Error, Debug)]
pub enum OsmError {
    #[error("invalid OSM import configuration: {0}")]
    ConfigurationError(String),
    #[error("failure reading .pbf file: {source}")]
    PbfLibError { source: osmpbf::Error },
    #[error("failure simplifying graph: {0}")]
    GraphSimplificationError(String),
    #[error("failure consolidating graph: {0}")]
    GraphConsolidationError(String),
    #[error("failure writing to file {0}: {1}")]
    CsvWriteError(String, csv::Error),
    #[error("attempting to get {0} adjacencies for node '{1}' not in graph")]
    AdjacencyMissingSourceNodeId(AdjacencyDirection, OsmNodeId),
    #[error("attempting to get destination node '{2}' for source node '{0}' via its {1} adjacency list not in graph")]
    AdjacencyWithSourceMissingDestinationNodeId(OsmNodeId, AdjacencyDirection, OsmNodeId),
    #[error("attempting to remove adjacency list entry for node '{0}': {1}")]
    AdjacencyRemovalError(OsmNodeId, String),
    #[error("attempting to get node '{0}' not in graph")]
    GraphMissingNodeId(OsmNodeId),
    #[error("attempting to get way '{0}' not in graph")]
    GraphMissingWayId(OsmWayId),
    #[error("{0}")]
    GraphModificationError(String),
    #[error("structure of OSM data is invalid: {0}")]
    InvalidOsmData(String),
    #[error("pbf does not contain any OSM 'node' elements")]
    NoNodesFound,
    #[error("pbf does not contain any OSM 'way' elements")]
    NoWaysFound,
    #[error("unable to deserialize WKT into geometry: {0}")]
    InvalidWKT(String),
    #[error("Geometry of WKT is not a valid extent: {0}")]
    InvalidExtentWKT(String),
    #[error("{0}")]
    InternalError(String),
}
