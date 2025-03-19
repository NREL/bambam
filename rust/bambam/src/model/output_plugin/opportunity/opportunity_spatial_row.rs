use geo::{Centroid, ConvexHull, Geometry, MultiPoint, Point};
use rstar::{primitives::PointWithData, PointDistance, RTreeObject, AABB};

/// An object stored in an opportunity spatial index which can be found
/// by a spatial query against its geometry and stores the lookup index
/// for the corresponding opportunity vector.
pub struct OpportunitySpatialRow {
    pub geometry: Geometry,
    pub index: usize,
}

impl OpportunitySpatialRow {
    /// creates a dummy query [`OpportunitySpatialRow`] object wrapping
    /// the query geometry for use with the [`rstar`] library.
    pub fn query(geometry: Geometry) -> OpportunitySpatialRow {
        OpportunitySpatialRow {
            geometry,
            index: 99999999999,
        }
    }
}

impl RTreeObject for OpportunitySpatialRow {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        match &self.geometry {
            Geometry::Point(g) => g.envelope(),
            Geometry::Polygon(g) => g.envelope(),
            Geometry::MultiPolygon(g) => {
                let points = g.iter().flat_map(|i| i.centroid()).collect();
                let mp = MultiPoint(points);
                mp.convex_hull().envelope()
            },
            _ => panic!("opportunities can only be paired with POINT, POLYGON, or MULTIPOLYGON geometry types")
        }
    }
}

impl PointDistance for OpportunitySpatialRow {
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        match self.geometry.centroid() {
            Some(c) => c.distance_2(point),
            None => 999999999.0,
        }
    }
}
