use super::mep_score_ops;
use crate::model::{
    fieldname,
    output_plugin::{
        isochrone::time_bin::TimeBin,
        mep_score::{
            spatial_intensities::{to_aabb, SpatialIntensities},
            Coefficients, Intensities, ModalIntensityConfig, OpportunityAccessRecord,
            SpatialCoefficients,
        },
    },
};
use geo::{Area, BooleanOps, Intersects};
use itertools::Itertools;
use routee_compass::{
    app::{compass::CompassComponentError, search::SearchAppResult},
    plugin::output::OutputPluginError,
};
use routee_compass_core::model::{
    state::{StateModel, StateVariable},
    unit::{AsF64, DistanceUnit, Time, TimeUnit},
};
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use wkt::ToWkt;

#[derive(Clone, Debug)]
pub enum ModalIntensityModel {
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
    Global {
        intensities: Intensities,
        // coefficients: Coefficients,
    },
    /// intensity values associated with spatial zones. for each included
    /// zone (represented by a polygon or multipolygon geometry), a collection
    /// of [`Intensities`].
    Zonal {
        // categories: Vec<String>,
        intensities: RTree<SpatialIntensities>,
        // coefficients: RTree<SpatialCoefficients>,
    },
    // / placeholder
    // / for all destinations, report the intensities for that location
    // / by multiplying the observed point-to-point state by the intensity rate
    PointToPointIntensities,
}

impl TryFrom<&ModalIntensityConfig> for ModalIntensityModel {
    type Error = CompassComponentError;

    fn try_from(value: &ModalIntensityConfig) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl ModalIntensityModel {
    /// the list and order of energy intensity categories for MEP 1 intensities.
    /// this is hard-coded to align with the order of MEP 1 coefficients "alpha",
    /// "beta", and "sigma".
    const MEP_1_INTENSITY_CATEGORIES: [(&'static str, &'static str); 3] =
        [("energy", "alpha"), ("time", "beta"), ("cost", "sigma")];

    /// uses this intensity model (and, optionally, the state model) to find the intensity value
    /// matching this opportunity instance.
    pub fn get_intensity_value(
        &self,
        mode_name: &str,
        intensity_category: &str,
        opportunity_instance: &OpportunityAccessRecord,
        state_model: Arc<StateModel>,
    ) -> Result<f64, OutputPluginError> {
        use ModalIntensityModel as M;
        use OpportunityAccessRecord as O;
        match (self, opportunity_instance) {
            (
                M::Global { intensities },
                O::Isochrone {
                    activity_type,
                    geometry,
                    time_bin,
                    count,
                },
            ) => {
                // global intensities and an isochrone opportunity access are the base case.
                // since we are using a time_bin, we handle the special case of time-based intensities using the
                // max value of the time bin. otherwise, the intensity value is found directly in the intensities HashMap.
                if intensity_category == fieldname::TRIP_TIME {
                    let time = time_bin.max_time(&TimeUnit::Minutes);
                    Ok(time.as_f64())
                } else {
                    let mode_intensities = intensities.get(mode_name).ok_or_else(|| {
                        OutputPluginError::OutputPluginFailed(format!(
                            "mode '{}' missing from modal intensities",
                            mode_name
                        ))
                    })?;
                    let intensity = mode_intensities.get(intensity_category).ok_or_else(|| {
                        OutputPluginError::OutputPluginFailed(format!(
                            "intensity category '{}' missing from modal intensities",
                            intensity_category
                        ))
                    })?;
                    intensity.get_intensity(&DistanceUnit::Miles)
                }
            }
            (
                M::Zonal { intensities },
                O::Isochrone {
                    activity_type,
                    geometry,
                    time_bin,
                    count,
                },
            ) => {
                // because we have zonal intensities, we need to find all intersecting zones with intensity values
                // and take the weighted average intensity by areal proportion.
                let envelope = to_aabb(geometry);
                let found_intensities: Vec<(f64, f64)> = intensities
                    .locate_in_envelope_intersecting(&envelope)
                    .filter(|i| i.geometry.intersects(opportunity_instance.geometry()))
                    .map(|i| {
                        // get all intensities + proportion them by the overlap area of the zones
                        let value = i.intensities.get_intensity_value(
                            mode_name,
                            intensity_category,
                            opportunity_instance,
                            state_model.clone(),
                        )?;
                        let intersection = opportunity_instance.intersection(&i.geometry)?;
                        let intersection_area = intersection.unsigned_area();
                        Ok((value, intersection_area as f64))
                    })
                    .collect::<Result<Vec<_>, OutputPluginError>>()?;

                match found_intensities[..] {
                    [] => Err(OutputPluginError::OutputPluginFailed(format!(
                        "no spatial intensities match opportunity accessed at geometry: {}",
                        geometry.to_wkt().to_string()
                    ))),
                    [(value, _)] => Ok(value),
                    _ => {
                        // weighted average of the intensity values by their proportional coverage of the isochrone
                        let numer = found_intensities.iter().map(|(v, w)| v * w).sum::<f64>();
                        let denom = found_intensities.iter().map(|(_, w)| w).sum::<f64>();
                        Ok(numer / denom)
                    }
                }
            }
            (
                M::Global { intensities },
                O::Point {
                    activity_type,
                    geometry,
                    state,
                },
            ) => {
                // future work: we observe the intensity directly here from the state vector.
                todo!()
            }
            (
                M::Zonal { intensities },
                O::Point {
                    activity_type,
                    geometry,
                    state,
                },
            ) => todo!(),
            (
                M::PointToPointIntensities,
                O::Point {
                    activity_type,
                    geometry,
                    state,
                },
            ) => {
                // for time, get the total travel time.
                // for energy + cost, get the total trip energy or cost and divide it
                // by the unit distance to get an intensity value
                todo!()
            }
            (M::PointToPointIntensities, O::Isochrone { .. }) => {
                Err(OutputPluginError::OutputPluginFailed(String::from(
                    "cannot use point-to-point intensities with isochrone opportunities",
                )))
            }
        }
    }

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
            ModalIntensityModel::Global {
                intensities,
                // coefficients,
            } => {
                // expect a time bin limit
                let max_time = time_bin
                    .ok_or_else(|| {
                        OutputPluginError::OutputPluginFailed(String::from(
                            "expected 'time_bin' on request for mep1 intensities",
                        ))
                    })?
                    .max_time as f64;
                let result = ModalIntensityModel::MEP_1_INTENSITY_CATEGORIES
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
