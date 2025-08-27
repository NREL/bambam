use geo::algorithm::concave_hull::ConcaveHull;
use geo::Geometry;
use geo::KNearestConcaveHull;
use geo::MultiPoint;
use routee_compass::plugin::output::OutputPluginError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum IsochroneAlgorithm {
    ConcaveHull { concavity: f32 },
    KNearestConcaveHull { k: u32 },
}

impl IsochroneAlgorithm {
    pub fn run(&self, mp: MultiPoint<f32>) -> Result<Geometry<f32>, OutputPluginError> {
        match self {
            IsochroneAlgorithm::ConcaveHull { concavity } => {
                if mp.len() < 3 {
                    Ok(Geometry::Polygon(geo::polygon!()))
                } else {
                    let hull = mp.concave_hull(*concavity);
                    Ok(Geometry::Polygon(hull))
                }
            }
            IsochroneAlgorithm::KNearestConcaveHull { k } => {
                let hull = mp.k_nearest_concave_hull(*k);
                Ok(Geometry::Polygon(hull))
            }
        }
    }
}
