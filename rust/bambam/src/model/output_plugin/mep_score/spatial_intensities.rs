use super::mep_score_ops;
use geo::{Point, Polygon};
use routee_compass::plugin::PluginError;
use rstar::{RTreeObject, AABB};
use std::collections::HashMap;

pub struct SpatialIntensities {
    polygon: Polygon,
    intensities: HashMap<String, HashMap<String, f64>>,
}

impl RTreeObject for SpatialIntensities {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            self.polygon.envelope().lower(),
            self.polygon.envelope().upper(),
        )
    }
}

impl SpatialIntensities {
    pub fn get_intensity(&self, mode: String, category: String) -> Result<f64, PluginError> {
        let intensity = mep_score_ops::get_intensity(&self.intensities, &mode, &category)?;
        Ok(intensity)
    }
}
