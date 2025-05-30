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
        geometry_format: GeometryFormat,
        category_column: String,
        count_column: Option<String>,
    },
    // OvertureMaps {
    //     geometry_format: String,
    //     category_column: String,
    // },
    WideFormat {
        geometry_format: GeometryFormat,
        /// maps fields of a [`csv::StringRecord`] to opportunity categories
        column_mapping: HashMap<String, String>,
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
            } => Ok(Self::LongFormat {
                geometry_format: geometry_format.clone(),
                category_column: category_column.clone(),
                count_column: count_column.clone(),
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
    pub fn read_geometry(
        &self,
        record: &StringRecord,
        headers: &HashMap<String, usize>,
    ) -> Result<Option<Point<f32>>, String> {
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
            } => {
                let name = get_activity_name(record, category_column, headers)?;
                let (_, count) = match count_column {
                    Some(col) => get_activity_count(record, &category_column, &name, headers)?,
                    None => (name.clone(), 1),
                };
                // let count = match count_column {
                //     Some(col) => match record.get(col) {
                //         Some(count_str) => count_str
                //             .parse::<u64>()
                //             .map_err(|e| format!("failure reading count column '{}': {}", col, e)),
                //         None => Err(format!("expected count column '{}' not found", col)),
                //     },
                //     None => Ok(Some(1)),
                // }?;
                Ok(HashMap::from([(name, count)]))
            }
            SourceFormat::WideFormat {
                geometry_format: _,
                column_mapping,
            } => column_mapping
                .iter()
                .map(|(category_column, category_name)| {
                    let (name, count) =
                        get_activity_count(record, category_column, category_name, headers)?;
                    Ok((name, count))
                })
                .collect::<Result<HashMap<String, u64>, String>>(),
        }
    }
}

/// used with long format file sources where each row contains a single
/// opportunity count
fn get_activity_name(
    record: &StringRecord,
    category_column: &str,
    headers: &HashMap<String, usize>,
) -> Result<String, String> {
    get_value(record, category_column, headers)
}

/// used with wide format file sources where each row contains aggregated
/// opportunity counts
fn get_activity_count(
    record: &StringRecord,
    count_column: &str,
    category_name: &str,
    headers: &HashMap<String, usize>,
) -> Result<(String, u64), String> {
    let count_str = get_value(record, count_column, headers)?;
    let count = count_str.parse::<u64>().map_err(|e| {
        format!(
            "unable to parse count '{}' for column '{}' as a non-negative integer",
            count_str, count_column
        )
    })?;
    Ok((category_name.to_string(), count))
}

fn get_value(
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

// fn top_level_om_category(map: &Map<String, Value>) -> Result<Option<String>, String> {
//     let primary = map
//         .get("primary")
//         .ok_or_else(|| String::from("row is not a JSON object with a 'primary' key"))?;
//     // 'primary' may be an array or null
//     match primary {
//         Value::Null => Ok(None),
//         Value::String(string) => Ok(Some(string.clone())),
//         _ => Err(format!(
//             "'primary' entry is not a string or null as expected, instead found {}",
//             primary
//         )),
//     }
// }
