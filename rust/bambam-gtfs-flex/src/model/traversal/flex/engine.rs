use crate::model::traversal::flex::zone_graph::{ZoneGraph, ZoneId, ZoneRecord};

use super::GtfsFlexConfig;

use chrono::NaiveDateTime;
use routee_compass_core::{
    model::{network::Vertex, traversal::TraversalModelError},
    util::geo::PolygonalRTree,
};

pub struct GtfsFlexEngine {
    pub graph: ZoneGraph,
    pub rtree: PolygonalRTree<f32, ZoneId>,
}

impl GtfsFlexEngine {
    pub fn valid_destination(
        &self,
        src_zone_id: &ZoneId,
        current_vertex: &Vertex,
        current_time: &NaiveDateTime,
    ) -> Result<bool, TraversalModelError> {
        let point = geo::Geometry::Point(geo::Point(current_vertex.coordinate.0));

        let zone_iter = self.rtree.intersection(&point).map_err(|e| {
            let msg = format!("failure looking up zone geometry from trip location: {e}");
            TraversalModelError::TraversalModelFailure(msg)
        })?;

        // check if any intersecting destination zones are valid for this trip
        for node in zone_iter {
            let is_valid = self
                .graph
                .valid_zonal_trip(src_zone_id, &node.data, current_time)?;
            if is_valid {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl TryFrom<GtfsFlexConfig> for GtfsFlexEngine {
    type Error = TraversalModelError;

    fn try_from(_config: GtfsFlexConfig) -> Result<Self, Self::Error> {
        // todo: use the zone records to create the graph between zones
        let zone_records: Vec<ZoneRecord> = vec![];
        let graph = ZoneGraph::try_from(zone_records.as_slice())?;

        // todo: load zone ids with geometries for the spatial index
        let zone_geometries: Vec<(geo::Geometry<f32>, ZoneId)> = vec![];
        let rtree = PolygonalRTree::new(zone_geometries).map_err(|e| {
            let msg = format!("failure building spatial index for GTFS Flex zones: {e}");
            TraversalModelError::BuildError(msg)
        })?;

        Ok(GtfsFlexEngine { graph, rtree })
    }
}
