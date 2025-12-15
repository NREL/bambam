use geo::Polygon;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wkt::ToWkt;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryGeometryFormat {
    Wkt,
    #[default]
    Wkb,
    GeoJson,
}

impl TryFrom<&str> for BoundaryGeometryFormat {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().trim() {
            "wkt" => Ok(Self::Wkt),
            "wkb" => Ok(Self::Wkb),
            "geojson" => Ok(Self::GeoJson),
            _ => Err(format!("unknown boundary geometry format '{value}'")),
        }
    }
}

impl std::fmt::Display for BoundaryGeometryFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BoundaryGeometryFormat::Wkt => "wkt",
            BoundaryGeometryFormat::Wkb => "wkb",
            BoundaryGeometryFormat::GeoJson => "geojson",
        };
        write!(f, "{s}")
    }
}

impl BoundaryGeometryFormat {
    pub fn serialize(&self, boundary: &Polygon) -> Result<Value, String> {
        match self {
            BoundaryGeometryFormat::Wkt => {
                let out = boundary.to_wkt().to_string();
                Ok(json![out])
            }
            BoundaryGeometryFormat::Wkb => {
                // Convert to WKB
                let mut out_bytes: Vec<u8> = vec![];
                wkb::writer::write_polygon(
                    &mut out_bytes,
                    boundary,
                    &wkb::writer::WriteOptions {
                        endianness: wkb::Endianness::BigEndian,
                    },
                );

                // Write to query
                let output = out_bytes
                    .iter()
                    .map(|b| format!("{b:02X?}"))
                    .collect::<Vec<String>>()
                    .join("");

                Ok(json![output])
            }
            BoundaryGeometryFormat::GeoJson => {
                let geometry = geojson::Geometry::from(&geo::Geometry::Polygon(boundary.clone()));
                let feature = geojson::Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: None,
                    foreign_members: None,
                };
                let result = serde_json::to_value(feature).map_err(|e| e.to_string())?;
                Ok(result)
            }
        }
    }
}
