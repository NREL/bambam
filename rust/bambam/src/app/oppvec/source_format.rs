use std::collections::HashMap;

use clap::{Subcommand, ValueEnum};
use csv::StringRecord;
use geo::{Geometry, HasDimensions, Point};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Clone, Debug, Subcommand)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SourceFormat {
    String {
        geometry_column: String,
        category_column: String,
    },
    OvertureMaps {
        geometry_column: Option<String>,
        category_column: Option<String>,
    },
    CoStar {
        latitude_column: Option<String>,
        longitude_column: Option<String>,
        // category_mapping: HashMap<(String, String), String>,
    },
}

impl std::fmt::Display for SourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFormat::String {
                geometry_column,
                category_column,
            } => {
                write!(
                    f,
                    "read geometry from '{}' column, category from '{}'",
                    geometry_column, category_column
                )
            }
            SourceFormat::OvertureMaps {
                geometry_column,
                category_column,
            } => {
                let geo_col = geometry_column
                    .clone()
                    .unwrap_or_else(|| properties::OVERTURE_MAPS_GEOMETRY.to_string());
                let cat_col = category_column
                    .clone()
                    .unwrap_or_else(|| properties::OVERTURE_CATEGORY_FIELD.to_string());
                write!(
                    f,
                    "read geometry from '{}' column, category from '{}' in OvertureMaps file",
                    geo_col, cat_col
                )
            }
            SourceFormat::CoStar {
                latitude_column: _,
                longitude_column: _,
                // category_mapping: _,
            } => write!(
                f,
                "read geometry from CoStar via longitude/latitude columns"
            ),
        }
    }
}

mod properties {
    use std::collections::HashMap;

    pub const OVERTURE_MAPS_GEOMETRY: &'static str = "geometry";
    pub const COSTAR_LATITUDE: &'static str = "latitude";
    pub const COSTAR_LONGITUDE: &'static str = "longitude";
    pub const OVERTURE_CATEGORY_FIELD: &'static str = "categories";
    pub const COSTAR_PROPERTYTYPE_FIELD: &'static str = "propertytype";
    pub const COSTAR_PROPERTYSUBTYPE_FIELD: &'static str = "propertysubtype";

    pub fn costar_category_mapping(propertytype: &str, propertysubtype: &str) -> Option<String> {
        match (propertytype, propertysubtype) {
            ("Health Care", _) => Some(String::from("healthcare")),
            ("Sports & Entertainment", _) => Some(String::from("entertainment")),
            ("Retail", _) => Some(String::from("retail")),
            ("Specialty", _) => Some(String::from("services")),
            (_, "Fast Food") => Some(String::from("food")),
            (_, "Restaurant") => Some(String::from("food")),
            (_, "Bar") => Some(String::from("food")),
            _ => None,
        }
    }
}

impl SourceFormat {
    fn description(&self) -> String {
        match self {
            SourceFormat::String {
                geometry_column: _,
                category_column: _,
            } => String::from("string read directly from CSV cell"),
            SourceFormat::OvertureMaps {
                geometry_column: _,
                category_column: _,
            } => String::from(
                r#"overture_maps category format is a json object with root parent at '.alternate[0]' position,
                    which is the most general category for this entry. for example, a
                    record with primary entry 'elementary_school' will have a '.alternate[0]' value of 'school'"#,
            ),
            SourceFormat::CoStar {
                latitude_column: _,
                longitude_column: _,
                // category_mapping: _,
            } => String::from("CoStar stores activity locations by property type and subtype"),
        }
    }

    pub fn read_geometry(
        &self,
        record: &StringRecord,
        headers: &HashMap<String, usize>,
    ) -> Result<Option<Point<f32>>, String> {
        // log::debug!("SourceFormat::read with '{}'", value);

        match self {
            SourceFormat::String {
                geometry_column,
                category_column: _,
            } => {
                let geometry_str = get_value(record, geometry_column, headers)?;
                if geometry_str.is_empty() {
                    return Ok(None);
                }
                let geometry: geo::Point<f32> = wkt::TryFromWkt::try_from_wkt_str(&geometry_str)
                    .map_err(|e| format!("invalid Point geometry '{}': {}", geometry_str, e))?;
                if geometry.is_empty() {
                    return Ok(None);
                }
                Ok(Some(geometry))
            }
            SourceFormat::OvertureMaps {
                geometry_column,
                category_column: _,
            } => {
                let geom_fieldname: String = geometry_column
                    .clone()
                    .unwrap_or_else(|| properties::OVERTURE_MAPS_GEOMETRY.to_string());
                let geometry_str = get_value(record, &geom_fieldname, headers)?;
                // log::debug!("read with value '{}'", value);
                let geometry: geo::Point<f32> = wkt::TryFromWkt::try_from_wkt_str(&geometry_str)
                    .map_err(|e| format!("invalid Point geometry '{}': {}", geometry_str, e))?;
                if geometry.is_empty() {
                    return Ok(None);
                }
                Ok(Some(geometry))
            }
            SourceFormat::CoStar {
                latitude_column,
                longitude_column,
                // category_mapping: _,
            } => {
                let lat_col = latitude_column
                    .clone()
                    .unwrap_or_else(|| properties::COSTAR_LATITUDE.to_string());
                let lon_col = longitude_column
                    .clone()
                    .unwrap_or_else(|| properties::COSTAR_LONGITUDE.to_string());
                let lon = get_value(record, &lon_col, headers)?;
                let lat = get_value(record, &lat_col, headers)?;
                match (lon.parse::<f32>(), lat.parse::<f32>()) {
                    (Ok(lon_f32), Ok(lat_f32)) => {
                        let geometry: geo::Point<f32> = geo::Point::new(lon_f32, lat_f32);
                        Ok(Some(geometry))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    pub fn read_category(
        &self,
        record: &StringRecord,
        headers: &HashMap<String, usize>,
    ) -> Result<Option<String>, String> {
        // log::debug!("SourceFormat::read with '{}'", value);

        match self {
            SourceFormat::String {
                geometry_column,
                category_column,
            } => get_value(record, &category_column, headers).map(Some),
            SourceFormat::OvertureMaps {
                geometry_column,
                category_column,
            } => {
                let cat_fieldname: String = category_column
                    .clone()
                    .unwrap_or_else(|| properties::OVERTURE_CATEGORY_FIELD.to_string());
                let json_str = get_value(record, &cat_fieldname, headers)?;
                // log::debug!("read with value '{}'", value);
                let json: Value = serde_json::from_str(&json_str).map_err(|e| format!("{}", e))?;
                match json {
                    Value::Null => Ok(None),
                    Value::Object(map) => {
                        top_level_om_category(&map).map_err(|e| format!("{}: {}", e, json_str))
                    }
                    _ => Err(format!("value is not a JSON object or null: {}", json_str)),
                }
            }
            SourceFormat::CoStar {
                latitude_column,
                longitude_column,
                // category_mapping,
            } => {
                let p_type = get_value(record, properties::COSTAR_PROPERTYTYPE_FIELD, headers)?;
                let p_subtype =
                    get_value(record, properties::COSTAR_PROPERTYSUBTYPE_FIELD, headers)?;
                Ok(properties::costar_category_mapping(&p_type, &p_subtype))
            }
        }
    }
}

fn get_value(
    record: &StringRecord,
    geometry_column: &str,
    headers: &HashMap<String, usize>,
) -> Result<String, String> {
    let column_index = headers
        .get(geometry_column)
        .ok_or_else(|| format!("column name '{}' missing from CSV", geometry_column))?;
    let record_value = record.get(*column_index).ok_or_else(|| {
        format!(
            "row missing index '{}' for '{}' column",
            column_index, geometry_column
        )
    })?;
    Ok(record_value.to_string())
}

fn top_level_om_category(map: &Map<String, Value>) -> Result<Option<String>, String> {
    let primary = map
        .get("primary")
        .ok_or_else(|| String::from("row is not a JSON object with a 'primary' key"))?;
    // 'primary' may be an array or null
    match primary {
        Value::Null => Ok(None),
        Value::String(string) => Ok(Some(string.clone())),
        _ => Err(format!(
            "'primary' entry is not a string or null as expected, instead found {}",
            primary
        )),
    }
}
