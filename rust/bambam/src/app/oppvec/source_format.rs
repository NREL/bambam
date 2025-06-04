use std::collections::HashMap;

use clap::{Subcommand, ValueEnum};
use csv::StringRecord;
use geo::{Geometry, HasDimensions, Point};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{default, geometry_format::GeometryFormat, SourceFormatConfig};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SourceFormat {
    LongFormat {
        /// type of geometry data on each row
        geometry_format: GeometryFormat,
        /// column containing category name for the row
        category_column: String,
        /// optional column containing activity counts per row. if not provided,
        /// each row will be counted for 1 activity.
        count_column: Option<String>,
        /// mapping from category name to activity type name(s). must not be empty.
        category_mapping: HashMap<String, Vec<String>>,
    },
    // OvertureMaps {
    //     geometry_format: String,
    //     category_column: String,
    // },
    WideFormat {
        geometry_format: GeometryFormat,
        /// maps fields of a [`csv::StringRecord`] to opportunity categories
        column_mapping: HashMap<String, Vec<String>>,
        // category_mapping: HashMap<(String, String), String>,
    },
}

impl TryFrom<&SourceFormatConfig> for SourceFormat {
    type Error = String;

    fn try_from(config: &SourceFormatConfig) -> Result<Self, Self::Error> {
        match config {
            SourceFormatConfig::LongFormat {
                geometry_format,
                category_column,
                count_column,
                category_mapping,
            } => Ok(Self::LongFormat {
                geometry_format: geometry_format.clone(),
                category_column: category_column.clone(),
                count_column: count_column.clone(),
                category_mapping: category_mapping.clone(),
            }),
            // SourceFormatConfig::OvertureMaps {
            //     geometry_format,
            //     category_column,
            // } => {
            //     // let geometry_format: String = geometry_format
            //     //     .clone()
            //     //     .unwrap_or_else(|| default::OVERTURE_MAPS_GEOMETRY.to_string());
            //     // let category_column: String = category_column
            //     //     .clone()
            //     //     .unwrap_or_else(|| default::OVERTURE_CATEGORY_FIELD.to_string());
            //     // let result = Self::OvertureMaps {
            //     //     geometry_format,
            //     //     category_column,
            //     // };
            //     // Ok(result)
            //     todo!("should this variant exist? we are describing a 'from CSV' pipeline here, when does that ever happen from OvertureMaps Parquet data?")
            // }
            SourceFormatConfig::WideFormat {
                geometry_format,
                column_mapping,
            } => Ok(Self::WideFormat {
                geometry_format: geometry_format.clone(),
                column_mapping: column_mapping.clone(),
            }),
        }
    }
}

impl SourceFormat {
    pub fn activity_categories(&self) -> Vec<String> {
        match self {
            SourceFormat::LongFormat {
                category_mapping, ..
            } => category_mapping
                .values()
                .flatten()
                .dedup()
                .cloned()
                .collect_vec(),
            SourceFormat::WideFormat { column_mapping, .. } => column_mapping
                .values()
                .flatten()
                .dedup()
                .cloned()
                .collect_vec(),
        }
    }

    pub fn read_geometry(
        &self,
        record: &StringRecord,
        headers: &HashMap<String, usize>,
    ) -> Result<Option<Geometry<f32>>, String> {
        // log::debug!("SourceFormat::read with '{}'", value);

        let geometry_format = match self {
            SourceFormat::LongFormat {
                geometry_format, ..
            } => geometry_format,
            SourceFormat::WideFormat {
                geometry_format, ..
            } => geometry_format,
        };

        let geometry = geometry_format.get_geometry(record, headers)?;

        if geometry.is_empty() {
            return Ok(None);
        }
        Ok(Some(geometry))
    }

    pub fn get_counts_by_category(
        &self,
        record: &StringRecord,
        headers: &HashMap<String, usize>,
    ) -> Result<HashMap<String, u64>, String> {
        // log::debug!("SourceFormat::read with '{}'", value);

        match self {
            SourceFormat::LongFormat {
                geometry_format: _,
                category_column,
                count_column,
                category_mapping,
            } => {
                let upstream_category = get_value_from_record(record, category_column, headers)?;
                let activity_categories =
                    get_long_activity_names(&upstream_category, category_mapping)?;
                let count = match count_column {
                    Some(col) => get_count_from_record(record, col, headers),
                    None => Ok(1),
                }?;
                let result = activity_categories
                    .into_iter()
                    .map(|name| (name, count))
                    .collect::<HashMap<_, _>>();
                Ok(result)
            }
            SourceFormat::WideFormat {
                geometry_format: _,
                column_mapping,
            } => {
                let mut counts_by_category: HashMap<String, u64> = HashMap::new();
                for (category_column, category_names) in column_mapping.iter() {
                    let count = get_count_from_record(record, category_column, headers)?;
                    for category_name in category_names.iter() {
                        counts_by_category
                            .entry(category_name.clone())
                            .and_modify(|cnts| *cnts += count)
                            .or_insert(count);
                    }
                }
                Ok(counts_by_category)
            }
        }
    }
}

/// uses a column->activity type mapping to get the activity name for some row of long-format
/// activity data.
fn get_long_activity_names(
    category_name: &str,
    mapping: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    mapping
        .get(category_name)
        .cloned()
        .ok_or_else(|| format!("category '{}' missing from category mapping", category_name))
}

/// pulls out an activity count from a record by it's column name
fn get_count_from_record(
    record: &StringRecord,
    count_column: &str,
    headers: &HashMap<String, usize>,
) -> Result<u64, String> {
    let count_str = get_value_from_record(record, count_column, headers)?;
    if count_str.is_empty() {
        Ok(0)
    } else {
        let count = easy_parse_u64(&count_str)?;
        Ok(count)
    }
}

/// parses integers like 1 or 1.0 as unsigned integers, failing if the value cannot be
/// expressed (eventually) as a u64
fn easy_parse_u64(s: &str) -> Result<u64, String> {
    let r1 = s
        .parse::<u64>()
        .map_err(|e| format!("unable to parse count '{}' as a non-negative integer", s));
    r1.or_else(|e| match s.parse::<f64>().ok() {
        None => Err(e),
        Some(f) if f < 0.0 => Err(e),
        Some(f) => Ok(f as u64),
    })
}

/// pulls out a key/value pair's value from a record by it's column name.
fn get_value_from_record(
    record: &StringRecord,
    key: &str,
    headers: &HashMap<String, usize>,
) -> Result<String, String> {
    let column_index = headers
        .get(key)
        .ok_or_else(|| format!("column name '{}' missing from CSV", key))?;
    let record_value = record
        .get(*column_index)
        .ok_or_else(|| format!("row missing index '{}' for '{}' column", column_index, key))?;
    Ok(record_value.to_string())
}
