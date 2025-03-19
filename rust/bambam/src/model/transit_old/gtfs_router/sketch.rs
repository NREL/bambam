use std::path::Path;

use gtfs_structures::RawGtfs;
use routee_compass_core::model::network::{EdgeId, Graph, VertexId};
use skiplist::OrderedSkipList;

pub enum TripDeparture {
    ScheduledTrip,
    FrequencyTrip,
}

pub struct TransitEdge {
    pub edge_id: EdgeId,
    pub src_vertex_id: VertexId,
    pub dst_vertex_id: VertexId,
    pub trip_departures: OrderedSkipList<TripDeparture>,
}

pub struct TransitNetwork {
    pub edges: Vec<TransitEdge>,
}

pub struct GtfsProcessor {}

impl GtfsProcessor {
    pub fn read<P>(path: P, graph: &Graph) -> Result<Vec<TransitEdge>, String>
    where
        P: AsRef<Path>,
    {
        // processes GTFS archive into an internal transit representation
        // that includes all information (no calendar filtering)
        // which can be used for routing.
        todo!()
    }
}

#[cfg(test)]
mod sketch {
    #[test]
    pub fn test() {}
}
