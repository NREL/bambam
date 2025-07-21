use crate::model::{
    fieldname,
    output_plugin::{
        isochrone::time_bin::TimeBin,
        mep_score::{
            intensity_category::IntensityCategory, Intensities, IntensityValue,
            ModalIntensityConfig, ModeIntensities, WeightingFactors,
        },
        opportunity::OpportunityRecord,
    },
};
use geo::{Area, BooleanOps, Geometry, Intersects};
use geojson::GeoJson;
use itertools::Itertools;
use routee_compass::{
    app::{compass::CompassComponentError, search::SearchAppResult},
    plugin::output::OutputPluginError,
};
use routee_compass_core::util::geo::PolygonalRTree;
use routee_compass_core::{
    algorithm::search::SearchInstance,
    config::CompassConfigurationError,
    model::{
        state::{StateModel, StateVariable},
        unit::{AsF64, Convert, DistanceUnit, EnergyUnit, Time, TimeUnit},
    },
};
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, str::FromStr, sync::Arc};
use wkt::ToWkt;

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
    ///     "walk": { "energy": { "type": "energy", "value": 0.0}, "cost": { "type": "cost", "value": 0.0} },
    ///     "drive": { "energy": { "type": "energy", "value": 0.48}, "cost": { "type": "cost", "value": 0.9} }
    ///   },    
    /// }
    /// ```
    Global {
        intensities: Intensities,
    },
    /// intensity values associated with spatial zones. for each included
    /// zone (represented by a polygon or multipolygon geometry), a collection
    /// of [`Intensities`].
    Zonal {
        intensities: PolygonalRTree<f32, Intensities>,
    },
    // / placeholder
    // / for all destinations, report the intensities for that location
    // / by multiplying the observed point-to-point state by the intensity rate
    EndogenousIntensities {
        distance_unit: DistanceUnit,
    },
}

impl TryFrom<&ModalIntensityConfig> for ModalIntensityModel {
    type Error = CompassComponentError;

    fn try_from(value: &ModalIntensityConfig) -> Result<Self, Self::Error> {
        match value {
            ModalIntensityConfig::Global { intensities } => {
                let intensities_deserialized = intensities
                    .iter()
                    .map(|(k, v)| {
                        let inner = v
                            .iter()
                            .map(|(cat, value_config)| {
                                (cat.clone(), IntensityValue::from((cat, value_config)))
                            })
                            .collect::<HashMap<_, _>>();
                        (k.clone(), inner)
                    })
                    .collect::<HashMap<_, _>>();
                Ok(Self::Global {
                    intensities: intensities_deserialized,
                })
            }
            ModalIntensityConfig::Endogenous { per_distance_unit } => {
                Ok(Self::EndogenousIntensities {
                    distance_unit: per_distance_unit.unwrap_or(DistanceUnit::Miles),
                })
            }
            ModalIntensityConfig::Zonal {
                zonal_intensities_input_file,
            } => {
                let feature_collection =
                    read_geojson_feature_collection(zonal_intensities_input_file)?;
                let intensities_data: Vec<(Geometry<f32>, Intensities)> = feature_collection
                    .into_iter()
                    .map(feature_to_intensities)
                    .collect::<Result<_, CompassConfigurationError>>()?;
                let intensities = PolygonalRTree::new(intensities_data).map_err(|e| {
                    CompassConfigurationError::UserConfigurationError(format!(
                        "failure building spatial index from file {}: {}",
                        zonal_intensities_input_file, e
                    ))
                })?;
                Ok(Self::Zonal { intensities })
            }
        }
    }
}

impl ModalIntensityModel {
    /// uses this intensity model (and, optionally, the state model) to find the intensity value
    /// matching this opportunity instance.
    pub fn get_intensity_value(
        &self,
        mode_name: &str,
        intensity_category: &IntensityCategory,
        opportunity_instance: &OpportunityRecord,
        si: &SearchInstance,
    ) -> Result<f64, OutputPluginError> {
        use ModalIntensityModel as M;
        use OpportunityRecord as O;
        // extract all relevant properties of this opportunity instance
        let time = opportunity_instance.get_time(si.state_model.clone())?;
        let activity_type = opportunity_instance.get_activity_type();
        let geometry = opportunity_instance.get_geometry();

        // use a different lookup method depending on the source of intensity values
        match self {
            M::Global { intensities } => intensity_lookup(
                intensity_category,
                mode_name,
                intensities,
                activity_type,
                time,
            ),
            M::Zonal { intensities } => spatial_intensity_lookup(
                intensity_category,
                mode_name,
                intensities,
                activity_type,
                geometry,
                time,
            ),
            M::EndogenousIntensities { distance_unit } => {
                match opportunity_instance {
                    O::Aggregate { .. } => Err(OutputPluginError::OutputPluginFailed(String::from("cannot observe endogenous intensities with isochrone-based opportunity modeling"))),
                    O::Disaggregate { activity_type, state, .. } => {
                        endogenous_intensity_observation(mode_name, intensity_category, state, si, distance_unit)
                    },
                }
            },
        }
    }
}

/// process an isochrone-based opportunity with global intensities
fn intensity_lookup(
    intensity_category: &IntensityCategory,
    mode_name: &str,
    intensities: &Intensities,
    activity_type: &str,
    time: (Time, TimeUnit),
) -> Result<f64, OutputPluginError> {
    // global intensities and an isochrone opportunity access are the base case.
    // since we are using a time_bin, we handle the special case of time-based intensities using the
    // max value of the time bin. otherwise, the intensity value is found directly in the intensities HashMap.
    match intensity_category {
        IntensityCategory::Time => {
            let (t, tu) = time;
            let mut t_cow = Cow::Owned(t);
            tu.convert(&mut t_cow, &TimeUnit::Minutes)
                .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
            Ok(t_cow.as_f64())
        }
        _ => {
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
            let value = intensity.get_value();
            Ok(value)
            // intensity.get_intensity(&DistanceUnit::Miles)
        }
    }
}

/// look up the intensity value(s) in a spatial dataset that match the provided opportunity location's geometry.
/// when needed, perform an averaging of the found values, using intersection area as the weighting for the average
/// calculation.
///
/// this applies to zonal and point-based geometry types.
fn spatial_intensity_lookup(
    intensity_category: &IntensityCategory,
    mode_name: &str,
    intensities: &PolygonalRTree<f32, Intensities>,
    activity_type: &str,
    geometry: &geo::Geometry<f32>,
    time: (Time, TimeUnit),
) -> Result<f64, OutputPluginError> {
    // because we have zonal intensities, we need to find all intersecting zones with intensity values
    // and take the weighted average intensity by areal proportion.
    let intersecting_zones_by_area = intensities
        .intersection_with_overlap_area(geometry)
        .map_err(|e| {
            OutputPluginError::OutputPluginFailed(format!(
                "failure during spatial intensity lookup: {}",
                e
            ))
        })?;
    let found_intensities: Vec<(f64, f64)> = intersecting_zones_by_area
        .into_iter()
        .filter(|(node, _)| node.geometry.intersects(geometry))
        .map(|(node, area)| {
            // actually look up the intensity value we are trying to find
            let value = intensity_lookup(
                intensity_category,
                mode_name,
                &node.data,
                activity_type,
                time,
            )?;
            Ok((value, area as f64))
        })
        .collect::<Result<Vec<_>, OutputPluginError>>()?;

    match found_intensities[..] {
        [] => Err(OutputPluginError::OutputPluginFailed(format!(
            "no spatial intensities match opportunity accessed at geometry: {}",
            geometry.to_wkt()
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

/// observes some traversal dimension and convert it to an intensity, a per-passenger, per-distance-unit rate.
fn endogenous_intensity_observation(
    mode_name: &str,
    intensity_category: &IntensityCategory,
    state: &[StateVariable],
    si: &SearchInstance,
    distance_unit: &DistanceUnit,
) -> Result<f64, OutputPluginError> {
    match intensity_category {
        IntensityCategory::Time => {
            let state_model = si.state_model.clone();
            let (time, _) = state_model
                .get_time(state, fieldname::TRIP_TIME, Some(&TimeUnit::Minutes))
                .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
            // according to the MEP methodology, this value is used directly as minutes
            Ok(time.as_f64())
        }
        IntensityCategory::Cost => {
            // grab trip total cost from the end of the trip using the cost model
            let cost = get_total_cost(state, si)?;
            per_distance_intensity(cost, state, si, distance_unit)
        }
        IntensityCategory::Energy => {
            let state_model = si.state_model.clone();
            let (energy, _) = state_model
                .get_energy(
                    state,
                    fieldname::TRIP_ENERGY,
                    Some(&EnergyUnit::KilowattHours),
                )
                .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
            per_distance_intensity(energy.as_f64(), state, si, distance_unit)
        }
    }
}

/// helper function that performs polygon/multipolygon intersection on two geometries or
/// reports an error due to mixing of invalid Geometry variants
fn intersect_polygonal(
    a: &geo::Geometry<f32>,
    b: &geo::Geometry<f32>,
) -> Result<geo::Geometry<f32>, OutputPluginError> {
    match (a, b) {
        (geo::Geometry::Polygon(a), geo::Geometry::Polygon(b)) => {
            let result = a.intersection(b);
            Ok(geo::Geometry::MultiPolygon(result))
        }
        (geo::Geometry::Polygon(a), geo::Geometry::MultiPolygon(b)) => {
            let result = a.intersection(b);
            Ok(geo::Geometry::MultiPolygon(result))
        }
        (geo::Geometry::MultiPolygon(a), geo::Geometry::Polygon(b)) => {
            let result = a.intersection(b);
            Ok(geo::Geometry::MultiPolygon(result))
        }
        (geo::Geometry::MultiPolygon(a), geo::Geometry::MultiPolygon(b)) => {
            let result = a.intersection(b);
            Ok(geo::Geometry::MultiPolygon(result))
        }
        _ => Err(OutputPluginError::OutputPluginFailed(format!(
            "attempting intersection with invalid geometry types: \n{}\n{}",
            a.to_wkt(),
            b.to_wkt()
        ))),
    }
}

// helper to convert an observed trip metric to a value-per-unit-distance metric (per passenger mile, etc)
fn per_distance_intensity(
    value: f64,
    state: &[StateVariable],
    si: &SearchInstance,
    distance_unit: &DistanceUnit,
) -> Result<f64, OutputPluginError> {
    // grab trip total distance using the state model, in the target unit.
    let (distance, _) = si
        .state_model
        .clone()
        .get_distance(state, fieldname::TRIP_DISTANCE, Some(distance_unit))
        .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
    let denom = distance.as_f64();
    if denom == 0.0 {
        Ok(0.0)
    } else {
        let value_per_dist = value / denom;
        Ok(value_per_dist)
    }
}

/// grab trip total cost from the end of the trip using the cost model
fn get_total_cost(state: &[StateVariable], si: &SearchInstance) -> Result<f64, OutputPluginError> {
    let cost_object = si
        .cost_model
        .clone()
        .serialize_cost(state, si.state_model.clone())
        .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
    let cost_json = cost_object.get("total_cost").ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(String::from(
            "could not find 'total_cost' in serialized edge cost",
        ))
    })?;
    let cost = cost_json.as_f64().ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "could not deserialize 'total_cost' from JSON into f64: {:?}",
            cost_json
        ))
    })?;
    Ok(cost)
}

/// helper to read a FeatureCollection from a file
fn read_geojson_feature_collection(
    input_file: &str,
) -> Result<geojson::FeatureCollection, CompassConfigurationError> {
    let contents = std::fs::read_to_string(input_file).map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "unable to load file {}: {}",
            input_file, e
        ))
    })?;
    let dataset = GeoJson::from_str(&contents).map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "failed to read file {} as GeoJSON: {}",
            input_file, e
        ))
    })?;
    let feature_collection = match dataset {
        GeoJson::Geometry(_) => Err(CompassConfigurationError::UserConfigurationError(format!("GeoJSON intensities must be a FeatureCollection but found single 'Geometry' in file {}", input_file))),
        GeoJson::Feature(_) => Err(CompassConfigurationError::UserConfigurationError(format!("GeoJSON intensities must be a FeatureCollection but found single 'Feature' in file {}", input_file))),
        GeoJson::FeatureCollection(feature_collection) => Ok(feature_collection),
    }?;

    Ok(feature_collection)
}

/// helper to unpack a feature into the component parts required to insert
/// it into a spatial index of intensity values.
fn feature_to_intensities(
    f: geojson::Feature,
) -> Result<(Geometry<f32>, Intensities), CompassConfigurationError> {
    let id = match f.id {
        Some(geojson::feature::Id::String(s)) => s.to_string(),
        Some(geojson::feature::Id::Number(n)) => n.to_string(),
        None => String::from("<no id>"),
    };
    let geom = f.geometry.ok_or_else(|| {
        CompassConfigurationError::UserConfigurationError(format!(
            "feature {} has no geometry which is invalid",
            id
        ))
    })?;
    let geometry: geo::Geometry<f32> = geom.try_into().map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "failed to decode geometry for feature {}: {}",
            id, e
        ))
    })?;
    let props = f.properties.ok_or_else(|| {
        CompassConfigurationError::UserConfigurationError(format!(
            "feature {} has no properties which is invalid",
            id
        ))
    })?;
    let intensities: Intensities = props
        .into_iter()
        .map(|(k, v)| {
            let v: ModeIntensities = serde_json::from_value(v).map_err(|e| {
                CompassConfigurationError::UserConfigurationError(format!(
                    "feature {} entry for mode {} has invalid mode intensities: {}",
                    id, k, e
                ))
            })?;
            Ok((k, v))
        })
        .collect::<Result<Intensities, CompassConfigurationError>>()?;
    Ok((geometry, intensities))
}
