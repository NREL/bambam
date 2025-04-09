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
    },
    // OvertureMaps {
    //     geometry_column: Option<String>,
    //     category_column: Option<String>,
    // },
    WideFormat {
        geometry_format: GeometryFormat,
        /// maps fields of a [`csv::StringRecord`] to opportunity categories
        column_mapping: HashMap<String, String>,
    },
}

impl SourceFormatConfig {
    fn description(&self) -> String {
        match self {
            SourceFormatConfig::LongFormat {
                geometry_format: _,
                category_column: _,
            } => String::from("long format data contains one opportunity count per row"),
            // SourceFormatConfig::OvertureMaps {
            //     geometry_column: _,
            //     category_column: _,
            // } => String::from(
            //     r#"overture_maps category format is a json object with root parent at '.alternate[0]' position,
            //         which is the most general category for this entry. for example, a
            //         record with primary entry 'elementary_school' will have a '.alternate[0]' value of 'school'"#,
            // ),
            SourceFormatConfig::WideFormat {
                geometry_format: _,
                column_mapping: _,
            } => String::from("wide format data contains aggregated opportunity counts per row"),
        }
    }
}

impl std::fmt::Display for SourceFormatConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFormatConfig::LongFormat {
                geometry_format,
                category_column,
            } => {
                write!(
                    f,
                    "geometry from '{}' and single activity with category from '{}'",
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
                    .map(|(k, v)| format!("'{}': '{}'", k, v))
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
