use geo::Intersects;
use geo_types::Geometry;
use geojson::GeoJson;
use routee_compass::{
    app::{compass::CompassComponentError, search::SearchAppResult},
    plugin::{output::OutputPluginError, PluginError},
};
use routee_compass_core::{config::CompassConfigurationError, util::geo::PolygonalRTree};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::format, hash::Hash, str::FromStr};
use tokio::net::tcp::ReuniteError;
use wkt::{ToWkt, TryFromWkt};

use crate::model::output_plugin::mep_score::ActivityFrequenciesConfig;

pub struct Frequencies {
    frequencies: HashMap<String, f64>,
    frequency_sum: f64,
}

pub enum ActivityFrequencies {
    GlobalFrequencies(Frequencies),
    ZonalFrequencies {
        frequencies: PolygonalRTree<f32, Frequencies>,
    },
}

impl TryFrom<&ActivityFrequenciesConfig> for ActivityFrequencies {
    type Error = CompassComponentError;

    fn try_from(value: &ActivityFrequenciesConfig) -> Result<Self, Self::Error> {
        match value {
            ActivityFrequenciesConfig::GlobalFrequencies { frequencies } => {
                let frequency_sum: f64 = frequencies.values().sum();
                if frequency_sum == 0.0 {
                    let err: PluginError = OutputPluginError::BuildFailed(String::from(
                        "sum of activity frequencies cannot be zero",
                    ))
                    .into();
                    Err(err.into())
                } else {
                    Ok(ActivityFrequencies::GlobalFrequencies(Frequencies {
                        frequencies: frequencies.clone(),
                        frequency_sum,
                    }))
                }
            }
            ActivityFrequenciesConfig::ZonalFrequencies {
                activity_frequencies_input_file,
            } => {
                let feature_collection =
                    read_geojson_feature_collection(activity_frequencies_input_file)?;

                let frequencies_data: Vec<(Geometry<f32>, Frequencies)> = feature_collection
                    .into_iter()
                    .map(feature_to_frequencies)
                    .collect::<Result<_, CompassComponentError>>()?;

                let frequencies = PolygonalRTree::new(frequencies_data).map_err(|e| CompassConfigurationError::UserConfigurationError(format!("failure building spatial index from file {activity_frequencies_input_file} : {e}")))?;

                Ok(Self::ZonalFrequencies { frequencies })
            }
        }
    }
}

impl ActivityFrequencies {
    pub fn get_frequency_term(
        &self,
        activity_type: &str,
        location: Option<&geo::Geometry<f32>>,
    ) -> Result<f64, OutputPluginError> {
        match self {
            ActivityFrequencies::GlobalFrequencies(Frequencies {
                frequencies,
                frequency_sum,
            }) => {
                let freq = frequencies.get(activity_type).ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "global frequencies missing activity type {activity_type}"
                    ))
                })?;

                Ok(*freq / *frequency_sum)
            }
            ActivityFrequencies::ZonalFrequencies { frequencies } => {
                if let Some(geometry) = location {
                    let intersecting_zones_by_area = frequencies
                        .intersection_with_overlap_area(geometry)
                        .map_err(|e| {
                            OutputPluginError::OutputPluginFailed(format!(
                                "failure during zonal frequency lookup: {e}"
                            ))
                        })?;

                    let found_intensities: Vec<(f64, f64)> = intersecting_zones_by_area
                        .into_iter()
                        .filter(|(node, _)| node.geometry.intersects(geometry))
                        .map(|(node, area)| {
                            // look up and calculate the activity frequency
                            let freq =
                                node.data.frequencies.get(activity_type).ok_or_else(|| {
                                    OutputPluginError::OutputPluginFailed(format!(
                                        "global frequencies missing activity type {activity_type}"
                                    ))
                                })?;
                            let frequency_sum = node.data.frequency_sum;
                            Ok((*freq / frequency_sum, area as f64))
                        })
                        .collect::<Result<Vec<_>, OutputPluginError>>()?;

                    match found_intensities[..] {
                        [] => Err(OutputPluginError::OutputPluginFailed(format!(
                            "no zonal frequencies match opportunity accessed at geometry: {}",
                            geometry.to_wkt()
                        ))),
                        [(value, _)] => Ok(value),
                        _ => {
                            // weighted average of the activity frequencies by
                            // their proportional coverage of the isochrone
                            let numer = found_intensities.iter().map(|(v, w)| v * w).sum::<f64>();
                            let denom = found_intensities.iter().map(|(_, w)| w).sum::<f64>();
                            Ok(numer / denom)
                        }
                    }
                } else {
                    Err(OutputPluginError::OutputPluginFailed(
                        "Missing geometry for zonal frequency".to_string(),
                    ))
                }
            }
        }
    }
}

/// helper to read a FeatureCollection from a file
fn read_geojson_feature_collection(
    input_file: &str,
) -> Result<geojson::FeatureCollection, CompassConfigurationError> {
    let contents = std::fs::read_to_string(input_file).map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "unable to load file {input_file}: {e}"
        ))
    })?;
    let dataset = GeoJson::from_str(&contents).map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "failed to read file {input_file} as GeoJSON: {e}"
        ))
    })?;
    let feature_collection = match dataset {
        GeoJson::Geometry(_) => Err(CompassConfigurationError::UserConfigurationError(format!("GeoJSON intensities must be a FeatureCollection but found single 'Geometry' in file {input_file}"))),
        GeoJson::Feature(_) => Err(CompassConfigurationError::UserConfigurationError(format!("GeoJSON intensities must be a FeatureCollection but found single 'Feature' in file {input_file}"))),
        GeoJson::FeatureCollection(feature_collection) => Ok(feature_collection),
    }?;

    Ok(feature_collection)
}

/// helper to unpack a feature into the component parts required to insert
/// it into a spatial index of frequency values.
fn feature_to_frequencies(
    feature: geojson::Feature,
) -> Result<(Geometry<f32>, Frequencies), CompassComponentError> {
    let id = match feature.id {
        Some(geojson::feature::Id::String(s)) => s.to_string(),
        Some(geojson::feature::Id::Number(n)) => n.to_string(),
        None => String::from("<no id>"),
    };

    let geom = feature.geometry.ok_or_else(|| {
        CompassConfigurationError::UserConfigurationError(format!(
            "feature {id} has no geometry which is invalid"
        ))
    })?;

    let geometry: geo::Geometry<f32> = geom.try_into().map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "failed to decode geometry for feature {id}: {e}"
        ))
    })?;

    let properties = feature.properties.ok_or_else(|| {
        CompassConfigurationError::UserConfigurationError(format!(
            "feature {id} has no properties which is invalid"
        ))
    })?;

    let frequencies: HashMap<String, f64> = properties
        .into_iter()
        .map(|(key, value)| {
            let de_value: f64 = serde_json::from_value(value).map_err(|e| {
                CompassConfigurationError::UserConfigurationError(format!(
                    "feature {id} entry for mode {key} has invalid activity frequencies: {e}"
                ))
            })?;
            Ok((key, de_value))
        })
        .collect::<Result<HashMap<String, f64>, CompassConfigurationError>>()?;

    let frequency_sum: f64 = frequencies.values().sum();
    if frequency_sum == 0.0 {
        let err: PluginError = OutputPluginError::BuildFailed(String::from(
            "sum of activity frequencies cannot be zero",
        ))
        .into();
        Err(err.into())
    } else {
        Ok((
            geometry,
            Frequencies {
                frequencies,
                frequency_sum,
            },
        ))
    }
}
