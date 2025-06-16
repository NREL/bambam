use super::mep_score_ops;
use crate::model::output_plugin::mep_score::{
    modal_intensity_model::ModalIntensityModel, Intensities,
};
use geo::{Extremes, Point, Polygon};
use routee_compass::plugin::PluginError;
use rstar::{Envelope, RTreeObject, AABB};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct SpatialIntensities {
    pub geometry: geo::Geometry<f32>,
    pub intensities: ModalIntensityModel,
}

impl RTreeObject for SpatialIntensities {
    type Envelope = AABB<Point<f32>>;

    fn envelope(&self) -> Self::Envelope {
        to_aabb(&self.geometry)
    }
}

/// helper function to convert Geometries to envelopes
pub fn to_aabb(geometry: &geo::Geometry<f32>) -> AABB<Point<f32>> {
    let ext = geometry
        .extremes()
        .expect("spatial intensity geometry cannot be empty");
    let lower = Point::new(ext.x_min.coord.x, ext.y_min.coord.y);
    let upper = Point::new(ext.x_max.coord.x, ext.y_max.coord.y);
    AABB::from_corners(lower, upper)
}

// impl SpatialIntensities {
//     pub fn get_intensity(&self, mode: String, category: String) -> Result<f64, PluginError> {
//         let intensity = mep_score_ops::get_intensity(&self.intensities, &mode, &category)?;
//         Ok(intensity)
//     }
// }
