use crate::model::output_plugin::mep_score::{Intensities, IntensitiesConfig, WeightingFactors};
use routee_compass::{app::search::SearchAppResult, plugin::output::OutputPluginError};
use routee_compass_core::model::unit::DistanceUnit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ModalIntensityConfig {
    /// a set of global intensities as a nested lookup table
    ///
    /// # Example
    ///
    /// the following is a (serialized) version of global intensities with
    /// walk and drive mode information for energy and cost intensity.
    ///
    /// ```json
    /// {
    ///   "type": "global_intensities",
    ///   "intensities": {
    ///     "walk": { "energy": 0.0, "cost": 0.0, "time": 1.0 },
    ///     "drive": { "energy": 0.48, "cost": 0.9, "time": 1.0 }
    ///   },
    ///   "coefficients": {
    ///     "alpha": -0.5,
    ///     "beta": -0.08,
    ///     "sigma": -0.5
    ///   }
    /// }
    /// ```
    Global { intensities: IntensitiesConfig },
    /// intensity values associated with spatial zones. for each included
    /// zone (represented by a polygon or multipolygon geometry), a collection
    /// of [`Intensities`] (stored in the Feature::properties of a GeoJSON).
    Zonal {
        zonal_intensities_input_file: String,
    },
    // / for all destinations, report the intensities for that location
    // / by multiplying the observed point-to-point state by the intensity rate
    Endogenous {
        /// if specified, the per-passenger distance unit used when
        /// observing endogenous intensities. if none, [`DistanceUnit::Miles`] is used.
        per_distance_unit: Option<DistanceUnit>,
    },
}
