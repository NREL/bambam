use super::{default, geometry_format::GeometryFormat};
use clap::{Subcommand, ValueEnum};
use csv::StringRecord;
use geo::{Geometry, HasDimensions, Point};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SourceFormatConfig {
    LongFormat {
        geometry_format: GeometryFormat,
        category_column: String,
        count_column: Option<String>,
        category_mapping: HashMap<String, Vec<String>>,
    },
    // OvertureMaps {
    //     geometry_column: Option<String>,
    //     category_column: Option<String>,
    // },
    WideFormat {
        geometry_format: GeometryFormat,
        /// maps fields of a [`csv::StringRecord`] to opportunity categories
        column_mapping: HashMap<String, Vec<String>>,
    },
}

impl std::fmt::Display for SourceFormatConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFormatConfig::LongFormat {
                geometry_format,
                category_column,
                count_column,
                category_mapping,
            } => {
                write!(
                    f,
                    "geometry from '{}' and single activity row with category from '{}'",
                    geometry_format, category_column
                )
            }
            // SourceFormatConfig::OvertureMaps {
            //     geometry_column,
            //     category_column,
            // } => {
            //     let geo_col = geometry_column
            //         .clone()
            //         .unwrap_or_else(|| default::OVERTURE_MAPS_GEOMETRY.to_string());
            //     let cat_col = category_column
            //         .clone()
            //         .unwrap_or_else(|| default::OVERTURE_CATEGORY_FIELD.to_string());
            //     write!(
            //         f,
            //         "read geometry from '{}' column, category from '{}' in OvertureMaps file",
            //         geo_col, cat_col
            //     )
            // }
            SourceFormatConfig::WideFormat {
                geometry_format,
                column_mapping,
            } => {
                let cats_middle = column_mapping
                    .iter()
                    .map(|(k, v)| format!("'{}': '{:?}'", k, v))
                    .join(",");
                let cats = format!("{{{}}}", cats_middle);
                write!(
                    f,
                    "geometry from '{}' and category mapping: '{}'",
                    geometry_format, cats
                )
            }
        }
    }
}
