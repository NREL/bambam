use geo::{Convert, Geometry, MapCoords, TryConvert};
use geo_traits::to_geo::ToGeoGeometry;
use geojson;
use routee_compass::plugin::{output::OutputPluginError, PluginError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wkb;
use wkt::{ToWkt, TryFromWkt, Wkt};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsochroneOutputFormat {
    Wkt,
    Wkb,
    GeoJson,
}

impl IsochroneOutputFormat {
    pub fn empty_geometry(&self) -> Result<String, OutputPluginError> {
        let empty: Geometry<f32> = Geometry::Polygon(geo::polygon![]);
        self.serialize_geometry(&empty)
    }

    pub fn deserialize_geometry(&self, value: &Value) -> Result<Geometry<f32>, OutputPluginError> {
        match self {
            IsochroneOutputFormat::Wkt => {
                let wkt = value.as_str().ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "expected WKT string for geometry deserialization, found: {:?}",
                        value
                    ))
                })?;
                let g = Geometry::try_from_wkt_str(wkt).map_err(|e| OutputPluginError::OutputPluginFailed(format!("failure deserializing WKT geometry from output row due to: {} - WKT string: \"{}\"", e, wkt)))?;
                Ok(g)
            }
            IsochroneOutputFormat::Wkb => {
                let wkb_str = value.as_str().ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "expected WKB string for geometry deserialization, found: {:?}",
                        value
                    ))
                })?;
                // Decode hex string to bytes
                let wkb_bytes = hex::decode(wkb_str).map_err(|e| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "failed to decode WKB hex string: {} - WKB string: \"{}\"",
                        e, wkb_str
                    ))
                })?;
                // Read geometry as f64, then convert to f32
                let geom_trait = wkb::reader::read_wkb(&wkb_bytes).map_err(|e| OutputPluginError::OutputPluginFailed(format!(
                    "failure deserializing WKB geometry from output row due to: {} - WKB string: \"{}\"", e, wkb_str
                )))?;
                let geometry_f64 = geom_trait.to_geometry();
                let geometry_f32 = try_convert_f32(&geometry_f64)?;
                Ok(geometry_f32)
            }
            IsochroneOutputFormat::GeoJson => {
                let geojson_str = value.as_str().ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "expected string for geometry deserialization, found: {:?}",
                        value
                    ))
                })?;
                let geojson_obj = geojson_str.parse::<geojson::GeoJson>().map_err(|e| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "failure parsing GeoJSON from geometry string due to: {}, found: {:?}",
                        e, value
                    ))
                })?;
                let geometry = geo_types::Geometry::<f32>::try_from(geojson_obj).map_err(|e| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "failure converting GeoJSON to Geometry due to: {}",
                        e
                    ))
                })?;
                Ok(geometry)
            }
        }
    }

    pub fn serialize_geometry(
        &self,
        geometry: &Geometry<f32>,
    ) -> Result<String, OutputPluginError> {
        match self {
            IsochroneOutputFormat::Wkt => Ok(geometry.wkt_string()),
            IsochroneOutputFormat::Wkb => {
                let mut out_bytes = vec![];
                let geom: Geometry<f64> = geometry.try_convert().map_err(|e| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "unable to convert geometry from f32 to f64: {}",
                        e
                    ))
                })?;
                let write_options = wkb::writer::WriteOptions {
                    endianness: wkb::Endianness::BigEndian,
                };
                wkb::writer::write_geometry(&mut out_bytes, &geom, &write_options).map_err(
                    |e| {
                        OutputPluginError::OutputPluginFailed(format!(
                            "failed to write geometry as WKB: {}",
                            e
                        ))
                    },
                )?;

                Ok(out_bytes
                    .iter()
                    .map(|b| format!("{:02X?}", b))
                    .collect::<Vec<String>>()
                    .join(""))
            }
            IsochroneOutputFormat::GeoJson => {
                let geometry = geojson::Geometry::from(geometry);
                let feature = geojson::Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: None,
                    foreign_members: None,
                };
                let result = serde_json::to_value(feature)?;
                Ok(result.to_string())
            }
        }
    }
}

fn try_convert_f32(g: &Geometry<f64>) -> Result<Geometry<f32>, OutputPluginError> {
    let (min, max) = (f32::MIN as f64, f32::MAX as f64);
    g.try_map_coords(|geo::Coord { x, y }| {
        if x < min || max < x {
            Err(OutputPluginError::OutputPluginFailed(format!(
                "could not express x value '{}' as f32, exceeds range of possible values [{}, {}]",
                x, min, max
            )))
        } else if y < min || max < y {
            Err(OutputPluginError::OutputPluginFailed(format!(
                "could not express y value '{}' as f32, exceeds range of possible values [{}, {}]",
                y, min, max
            )))
        } else {
            let x32 = std::panic::catch_unwind(|| x as f32).map_err(|e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "could not express x value '{}' as f32",
                    x
                ))
            })?;
            let y32 = std::panic::catch_unwind(|| y as f32).map_err(|e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "could not express y value '{}' as f32",
                    x
                ))
            })?;
            Ok(geo::Coord { x: x32, y: y32 })
        }
    })
}
