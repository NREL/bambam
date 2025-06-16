use crate::model::output_plugin::isochrone::time_bin::TimeBin;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::model::state::StateVariable;
use serde::{Deserialize, Serialize};

/// properties of accessing some activity type from a grid cell origin location. comes in two flavors:
///
///   1. Isochrone - zonal (aggregate) access to a type of activity
///   2. Point     - access data for exactly one opportunity
///
/// the properties of this opportunity access influence the modal intensities, modal coefficients,
/// and activity frequencies selected for computing an access metric.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpportunityAccessRecord {
    Isochrone {
        activity_type: String,
        geometry: geo::Geometry<f32>,
        time_bin: TimeBin,
        count: u64,
    },
    Point {
        activity_type: String,
        geometry: geo::Geometry<f32>,
        state: Vec<StateVariable>,
    },
}

impl OpportunityAccessRecord {
    pub fn geometry(&self) -> &geo::Geometry<f32> {
        match self {
            OpportunityAccessRecord::Isochrone { geometry, .. } => geometry,
            OpportunityAccessRecord::Point { geometry, .. } => &geometry,
        }
    }

    pub fn intersection(
        &self,
        other: &geo::Geometry<f32>,
    ) -> Result<geo::Geometry<f32>, OutputPluginError> {
        todo!()
    }
}
