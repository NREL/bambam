// OSMInfo struct is used to store needed information for OSM data for utilization in WCI calculations (wci.rs)
// August 2025 EG

use bambam_osm::model::osm::graph::OsmWayDataSerializable;
use geo::prelude::*;
use geo::{Euclidean, LineString, Point};
use rstar::{PointDistance, RTreeObject, AABB};

#[derive(Clone)]
pub struct OSMInfo {
    pub geo: LineString<f32>,
    pub data: OsmWayDataSerializable,
    pub stop: bool,
    pub traf_sig: bool,
}

impl RTreeObject for OSMInfo {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        match self.geo.bounding_rect() {
            Some(bounding_box) => AABB::from_corners(
                [bounding_box.min().x, bounding_box.min().y],
                [bounding_box.max().x, bounding_box.max().y],
            ),
            None => AABB::from_corners([0.0, 0.0], [0.0, 0.0]),
        }
    }
}

impl PointDistance for OSMInfo {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let query_point = geo::Point::new(point[0], point[1]);
        let midpoint = self.geo.centroid();
        if let Some(midpoint) = midpoint {
            let distance = Euclidean::distance(&geo::Euclidean, &midpoint, &query_point);
            distance * distance
        } else {
            f32::MAX
        }
    }
}
