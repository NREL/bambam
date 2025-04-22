use super::mep_score_ops;
use crate::model::output_plugin::isochrone::time_bin::TimeBin;
use routee_compass::{app::search::SearchAppResult, plugin::output::OutputPluginError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ModalIntensityValues {
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
    Mep1Intensities {
        intensities: HashMap<String, HashMap<String, f64>>,
        coefficients: HashMap<String, f64>,
    },
    // SpatialIntensities {
    //     categories: Vec<String>,
    //     intensities: RTree<SpatialIntensities>,
    // },
    // / placeholder
    // / for all destinations, report the intensities for that location
    // / by multiplying the observed point-to-point state by the intensity rate
    // PointToPointIntensities,
}

impl ModalIntensityValues {
    /// the list and order of energy intensity categories for MEP 1 intensities.
    /// this is hard-coded to align with the order of MEP 1 coefficients "alpha",
    /// "beta", and "sigma".
    const MEP_1_INTENSITY_CATEGORIES: [(&'static str, &'static str); 3] =
        [("energy", "alpha"), ("time", "beta"), ("cost", "sigma")];

    // pub fn get_intensity_row(
    //     &self,
    //     mode: &String,
    //     parent_vertex: &VertexId,
    //     search_branch: &SearchTreeBranch,
    // ) -> Result<Vec<f64>, OutputPluginError> {

    // }

    /// gets rows of intensity values. for global modal intensities, this will be
    /// a single row. for point-to-point MEP, we get one row per destination, tagged
    /// by it's VertexId found in the SearchAppResult.tree.
    ///
    /// # Arguments
    ///
    /// * `mode`     - mode traveled, such as "drive", "walk"
    /// * `result`   - search result
    ///
    /// # Returns
    ///
    /// A collection of modal intensity rows which have been scaled by their
    /// per-passenger mile rates. these are optionally tagged by their associated
    /// key in the SearchAppResult.tree lookup if they are point-to-point results.
    pub fn get_intensity_vector(
        &self,
        mode: &String,
        time_bin: Option<&TimeBin>,
        _id: Option<usize>,
        _result: &SearchAppResult,
    ) -> Result<Vec<f64>, OutputPluginError> {
        match self {
            // ModalIntensityValues::PointToPointIntensities => todo!(),
            // ModalIntensityValues::SpatialIntensities => todo!(),
            ModalIntensityValues::Mep1Intensities {
                intensities,
                coefficients,
            } => {
                // expect a time bin limit
                let max_time = time_bin
                    .ok_or_else(|| {
                        OutputPluginError::OutputPluginFailed(String::from(
                            "expected 'time_bin' on request for mep1 intensities",
                        ))
                    })?
                    .max_time as f64;
                let result = ModalIntensityValues::MEP_1_INTENSITY_CATEGORIES
                    .iter()
                    .map(|(category, coefficient)| {
                        let coef_value =
                            coefficients.get(&coefficient.to_string()).ok_or_else(|| {
                                OutputPluginError::OutputPluginFailed(format!(
                                    "expected coefficient {} for intensity category {} not found",
                                    coefficient, category
                                ))
                            })?;
                        if *category == "time" {
                            let intensity_weight = max_time * coef_value;
                            Ok(intensity_weight)
                        } else {
                            let intensity = mep_score_ops::get_intensity(
                                intensities,
                                mode,
                                &category.to_string(),
                            )?;

                            let intensity_weight = intensity * coef_value;
                            Ok(intensity_weight)
                        }
                    })
                    .collect::<Result<Vec<f64>, OutputPluginError>>()?;

                Ok(result)
            }
        }
    }
}
