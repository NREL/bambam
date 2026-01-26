use geo::Centroid;
use geo::{Geometry, Point};
use rstar::RTreeObject;
use rstar::AABB;

/// creates an envelope from a geometry using assumptions that
/// - points, linestrings, polygons can have their bboxes be their envelopes
/// - other geometry types can use their centroids
///
/// since a centroid may not exist (for example, empty geometries), the result may be None
///
/// # Arguments
///
/// * `geometry` - value to create an envelope from
///
/// # Returns
///
/// * an envelope if possible, otherwise None
pub fn get_centroid_as_envelope(geometry: &Geometry<f32>) -> Option<AABB<Point<f32>>> {
    match geometry {
        Geometry::Point(g) => Some(g.envelope()),
        Geometry::Line(g) => Some(g.envelope()),
        Geometry::LineString(g) => Some(g.envelope()),
        Geometry::Polygon(g) => Some(g.envelope()),
        Geometry::MultiPoint(g) => g.centroid().map(AABB::from_point),
        Geometry::MultiLineString(g) => g.centroid().map(AABB::from_point),
        Geometry::MultiPolygon(g) => g.centroid().map(AABB::from_point),
        Geometry::GeometryCollection(g) => g.centroid().map(AABB::from_point),
        Geometry::Rect(g) => Some(AABB::from_point(g.centroid())),
        Geometry::Triangle(g) => Some(AABB::from_point(g.centroid())),
    }
}
