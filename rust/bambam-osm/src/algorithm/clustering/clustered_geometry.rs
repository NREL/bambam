use crate::model::osm::graph::OsmNodeId;
use geo::{Intersects, Polygon};
use itertools::Itertools;

pub struct ClusteredGeometry(Vec<(OsmNodeId, Polygon<f32>)>);

impl ClusteredGeometry {
    pub fn new(geometry_index: OsmNodeId, polygon: Polygon<f32>) -> ClusteredGeometry {
        ClusteredGeometry(vec![(geometry_index, polygon)])
    }

    pub fn polygons(&self) -> Vec<&Polygon<f32>> {
        self.0.iter().map(|(_, p)| p).collect_vec()
    }

    pub fn ids(&self) -> Vec<OsmNodeId> {
        self.0.iter().map(|(idx, _)| *idx).collect_vec()
    }

    pub fn merge_and_sort_with(&mut self, other: &ClusteredGeometry) {
        self.0.extend(other.0.clone());
        self.0.sort_by_key(|(id, _)| *id);
    }

    pub fn intersects(&self, other: &Polygon<f32>) -> bool {
        for (_, p) in self.0.iter() {
            if p.intersects(other) {
                return true;
            }
        }
        false
    }
}
