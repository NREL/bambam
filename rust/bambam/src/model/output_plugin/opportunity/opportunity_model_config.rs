use super::opportunity_source::OpportunitySource;
use super::{
    opportunity_model::OpportunityModel, opportunity_table_orientation::OpportunityTableOrientation,
};
use csv::{ReaderBuilder, StringRecord};
use flate2::read::GzDecoder;
use geo::{Centroid, Convert, Point};
use itertools::Itertools;
use kdam::{tqdm, Bar, BarExt};
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::model::network::Vertex;
use routee_compass_core::util::fs::{fs_utils, read_utils};
use rstar::primitives::GeomWithData;
use rstar::RTree;
use serde::de;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::BufReader};
use wkt::ToWkt;

/// Configuration object for building an [`OpportunityModel`] called by it's
/// [`routee_compass::plugin::output::OutputPluginBuilder`]. See [`OpportunityModel`]
/// for algorithm implementation details.
#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum OpportunityModelConfig {
    /// this collection of opportunities comes from a file source.
    #[serde(rename = "file")]
    FileSource {
        opportunity_input_file: String,
        activity_column_names: Vec<String>,
        table_orientation: OpportunityTableOrientation,
    },
    /// this collection of opportunities comes from an API.
    #[serde(rename = "api")]
    ApiSource {
        vertex_input_file: String,
        opportunity_source: OpportunitySource,
        activity_column_names: Vec<String>,
        table_orientation: OpportunityTableOrientation,
    },
    Combined {
        models: Vec<Box<OpportunityModelConfig>>,
    },
}

impl OpportunityModelConfig {
    pub fn build(&self) -> Result<OpportunityModel, OutputPluginError> {
        match self {
            OpportunityModelConfig::FileSource {
                opportunity_input_file,
                activity_column_names,
                table_orientation,
            } => {
                // set up to read file
                let f = File::open(opportunity_input_file).map_err(|e| {
                    OutputPluginError::BuildFailed(format!(
                        "failed reading opportunities from {}: {}",
                        opportunity_input_file, e
                    ))
                })?;
                let r: Box<dyn std::io::Read> = if fs_utils::is_gzip(opportunity_input_file) {
                    Box::new(BufReader::new(GzDecoder::new(f)))
                } else {
                    Box::new(f)
                };
                let mut reader = ReaderBuilder::new().has_headers(true).from_reader(r);

                // track column names and their indices, and validate that all expected column names are present in the CSV header
                let mut column_lookup: HashMap<String, usize> = HashMap::new();
                reader
                    .headers()
                    .map_err(|e| {
                        OutputPluginError::BuildFailed(format!(
                            "failure reading headers from {}: {}",
                            opportunity_input_file, e
                        ))
                    })?
                    .iter()
                    .enumerate()
                    .for_each(|(index, column)| {
                        column_lookup.insert(column.to_string(), index);
                    });
                for col in activity_column_names.iter() {
                    if !column_lookup.contains_key(col) {
                        return Err(OutputPluginError::BuildFailed(format!(
                            "file {} is missing expected column {}",
                            opportunity_input_file, col
                        )));
                    }
                }

                // deserialize each CSV row, collecting the id and counts into a vector
                let mut activity_counts: Vec<Vec<f64>> = vec![];
                for row_result in reader.into_records() {
                    let row = row_result.map_err(|e| {
                        OutputPluginError::BuildFailed(format!(
                            "failure reading row from {}: {}",
                            opportunity_input_file, e
                        ))
                    })?;
                    let mut row_counts = vec![];
                    for col in activity_column_names.iter() {
                        let cnt = get_f64_from_row(&row, col, &column_lookup)?;
                        row_counts.push(cnt);
                    }
                    activity_counts.push(row_counts);
                }

                let result = OpportunityModel::Tabular {
                    activity_types: activity_column_names.to_owned(),
                    activity_counts,
                    table_orientation: table_orientation.to_owned(),
                };
                Ok(result)
            }
            OpportunityModelConfig::ApiSource {
                vertex_input_file,
                opportunity_source,
                activity_column_names,
                table_orientation,
            } => {
                let raw_dataset = opportunity_source
                    .generate_dataset(activity_column_names)
                    .map_err(OutputPluginError::OutputPluginFailed)?;
                let rtree_dataset = raw_dataset
                    .iter()
                    .enumerate()
                    .map(|(index, (g, _))| match g {
                        geo::Geometry::Point(p) => Ok(GeomWithData::new(*p, index)),
                        geo::Geometry::Polygon(p) => match p.centroid() {
                            Some(centroid) => Ok(GeomWithData::new(centroid, index)),
                            None => Err(OutputPluginError::OutputPluginFailed(format!(
                                "opportunity source geometries must have centroids, failed with {}",
                                p.to_wkt()
                            ))),
                        },
                        geo::Geometry::MultiPolygon(p) => match p.centroid() {
                            Some(centroid) => Ok(GeomWithData::new(centroid, index)),
                            None => Err(OutputPluginError::OutputPluginFailed(format!(
                                "opportunity source geometries must have centroids, failed with {}",
                                p.to_wkt()
                            ))),
                        },
                        _ => Err(OutputPluginError::OutputPluginFailed(format!(
                            "unsupported geometry, must be point, polygon, or multipolygon: {}",
                            g.to_wkt()
                        ))),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let rtree = RTree::bulk_load(rtree_dataset.to_vec());
                let vertices: Box<[Vertex]> = read_utils::from_csv(
                    &vertex_input_file,
                    true,
                    Some(
                        Bar::builder()
                            .desc("read vertices file for opportunity map matching")
                            .animation("fillup"),
                    ),
                    None,
                )
                .map_err(|e| {
                    OutputPluginError::BuildFailed(format!(
                        "failure reading vertices from {}: {}",
                        vertex_input_file, e
                    ))
                })?;

                // find which data point matches each vertex and record the id (index) of the match
                let match_iter = tqdm!(
                    vertices.iter(),
                    total = vertices.len(),
                    desc = "map match opportunities"
                );
                let match_by_vertex = match_iter
                    .map(|v| {
                        let point: Point<f64> = geo::Point(v.coordinate.0).convert();
                        rtree.nearest_neighbor(&point).map(|o| o.data)
                    })
                    .collect_vec();

                // build a lookup used for proportioning data into vertices which stores the counts
                // of vertices that match each identifier
                let proportion_groups = match_by_vertex
                    .iter()
                    .enumerate()
                    .chunk_by(|(_, match_id)| *match_id);
                let proportion_iter = tqdm!(
                    proportion_groups.into_iter(),
                    total = vertices.len(),
                    desc = "opportunities areal proportioning"
                );
                let mut proportion_lookup = HashMap::new();
                for (matching_geom_opt, matches) in proportion_iter {
                    let count = matches.collect_vec().len();
                    match matching_geom_opt {
                        None => {}
                        Some(id) => {
                            proportion_lookup.insert(id, count);
                        }
                    }
                }

                // ok! we can create our opportunity table now
                let n_acts = activity_column_names.len();
                let activity_counts = match_by_vertex
                    .iter()
                    .map(|id_option| match id_option {
                        None => Ok(vec![0.0; n_acts]),
                        Some(id) => {
                            let denom = proportion_lookup.get(id).ok_or_else(|| {
                                OutputPluginError::OutputPluginFailed(String::from(
                                    "internal error",
                                ))
                            })?;
                            let (_, raw_counts) = raw_dataset.get(*id).ok_or_else(|| {
                                OutputPluginError::OutputPluginFailed(String::from(
                                    "internal error",
                                ))
                            })?;
                            let counts = raw_counts
                                .iter()
                                .map(|count| *count / *denom as f64)
                                .collect_vec();
                            Ok(counts)
                        }
                    })
                    .collect::<Result<Vec<_>, OutputPluginError>>()?;

                let result = OpportunityModel::Tabular {
                    activity_types: activity_column_names.clone(),
                    activity_counts,
                    table_orientation: *table_orientation,
                };
                Ok(result)
            }
            OpportunityModelConfig::Combined { models } => {
                let models = models
                    .iter()
                    .map(|model| model.build().map(Box::new))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(OpportunityModel::Combined { models })
            }
        }
    }
}

/// gets a u32 from a CSV cell by column name, when also provided a lookup table
/// giving indices by column name.
fn get_u32_from_row(
    row: &StringRecord,
    col: &String,
    lookup: &HashMap<String, usize>,
) -> Result<u32, OutputPluginError> {
    let record_index = lookup.get(col).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!("file is missing expected column {}", col))
    })?;
    let value = row.get(*record_index).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "file column {} is missing from mapping but requested by the opportunity model",
            col
        ))
    })?;

    let number: u32 = value.parse().map_err(|e| {
        OutputPluginError::OutputPluginFailed(format!(
            "could not read {} as an integer: {}",
            value, e
        ))
    })?;
    Ok(number)
}

/// gets a u32 from a CSV cell by column name, when also provided a lookup table
/// giving indices by column name.
fn get_f64_from_row(
    row: &StringRecord,
    col: &String,
    lookup: &HashMap<String, usize>,
) -> Result<f64, OutputPluginError> {
    let record_index = lookup.get(col).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!("file is missing expected column {}", col))
    })?;
    let value = row.get(*record_index).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "file column {} is missing from mapping but requested by the opportunity model",
            col
        ))
    })?;

    let number: f64 = value.parse().map_err(|e| {
        OutputPluginError::OutputPluginFailed(format!(
            "could not read {} as an f64 (floating point value): {}",
            value, e
        ))
    })?;
    Ok(number)
}

/// gets a deserializable value from a CSV cell by column name, when also provided a lookup table
/// giving indices by column name.
///
/// used when we can't rely on serde for deserialization because the size of the row is not
/// known at compile time.
///
/// hey, this fails for my expected input 'failed to deserialize column retail - invalid type: string "0", expected u32'
/// i'll skip this generic approach and do a specialized u32 get operation
fn get_from_row<T>(
    row: &StringRecord,
    col: &String,
    lookup: &HashMap<String, usize>,
) -> Result<T, OutputPluginError>
where
    T: de::DeserializeOwned,
{
    let record_index = lookup.get(col).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!("file is missing expected column {}", col))
    })?;
    let value = row.get(*record_index).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "file column {} is missing from mapping but requested by the opportunity model",
            col
        ))
    })?;
    use de::IntoDeserializer;
    let result: Result<T, OutputPluginError> =
        T::deserialize(value.into_deserializer()).map_err(|e: de::value::Error| {
            OutputPluginError::OutputPluginFailed(format!(
                "failed to deserialize column {} - {}",
                col, e
            ))
        });
    result
}
