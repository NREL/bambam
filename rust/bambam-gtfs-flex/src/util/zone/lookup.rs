use crate::util::zone::{ZoneError, ZoneGraph, ZoneId, ZoneLookupConfig, ZoneRecord};

use chrono::NaiveDateTime;
use kdam::BarBuilder;
use routee_compass_core::{
    model::{frontier::FrontierModelError, network::Vertex, traversal::TraversalModelError},
    util::{fs::read_utils, geo::PolygonalRTree},
};

pub struct ZoneLookup {
    pub graph: ZoneGraph,
    pub rtree: PolygonalRTree<f32, ZoneId>,
}

impl ZoneLookup {
    /// is it valid to begin a trip in this zone at this time?
    pub fn valid_departure(
        &self,
        src_zone_id: &ZoneId,
        current_time: &NaiveDateTime,
    ) -> Result<bool, FrontierModelError> {
        self.graph.valid_departure(src_zone_id, current_time)
    }

    /// is it valid to end a trip that began at the src zone and reached this dst zone
    /// at this time?
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

impl TryFrom<&ZoneLookupConfig> for ZoneLookup {
    type Error = ZoneError;

    fn try_from(config: &ZoneLookupConfig) -> Result<Self, Self::Error> {
        let graph = read_records(&config.zone_record_input_file)?;

        // todo: load zone ids with geometries for the spatial index
        let rtree = read_geometries(&config.zone_geometry_input_file)?;

        Ok(ZoneLookup { graph, rtree })
    }
}

fn read_records(zone_record_input_file: &str) -> Result<ZoneGraph, ZoneError> {
    let bb = BarBuilder::default().desc("reading zone records");
    let zone_records: Box<[ZoneRecord]> =
        read_utils::from_csv(&zone_record_input_file, true, Some(bb), None).map_err(|e| {
            let msg = format!("failure reading zone records: {e}");
            TraversalModelError::BuildError(msg)
        })?;
    let graph = ZoneGraph::try_from(&zone_records[..])?;
    Ok(graph)
}

fn read_geometries(geometry_input_file: &str) -> Result<PolygonalRTree<f32, ZoneId>, ZoneError> {
    let zone_geometries: Vec<(geo::Geometry<f32>, ZoneId)> = todo!("read from the geojson");
    let rtree = PolygonalRTree::new(zone_geometries).map_err(|e| {
        let msg = format!("failure building spatial index for GTFS Flex zones: {e}");
        TraversalModelError::BuildError(msg)
    })?;
    Ok(rtree)
}
