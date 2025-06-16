use crate::model::output_plugin::mep_score::{Coefficients, Intensities};

use super::mep_score_ops;
use geo::{Point, Polygon};
use routee_compass::plugin::{output::OutputPluginError, PluginError};
use rstar::{RTreeObject, AABB};
use std::collections::HashMap;
use wkt::ToWkt;

#[derive(Clone, Debug)]
pub struct SpatialCoefficients {
    polygon: Polygon,
    coefficients: Coefficients,
}

impl RTreeObject for SpatialCoefficients {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            self.polygon.envelope().lower(),
            self.polygon.envelope().upper(),
        )
    }
}

impl SpatialCoefficients {
    pub fn get_coefficient(&self, mode: &str) -> Result<f64, OutputPluginError> {
        self.coefficients
            .get(mode)
            .ok_or_else(|| {
                OutputPluginError::OutputPluginFailed(format!(
                    "no spatial coefficient found for mode {} at location {}",
                    mode,
                    self.polygon.to_wkt().to_string()
                ))
            })
            .cloned()
    }
}
