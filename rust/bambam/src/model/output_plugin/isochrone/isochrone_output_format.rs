use geo::{Geometry, TryConvert};
use routee_compass::plugin::{output::OutputPluginError, PluginError};
use serde::{Deserialize, Serialize};
use wkb;
use wkt::ToWkt;

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
                wkb::writer::write_geometry(&mut out_bytes, &geom, wkb::Endianness::BigEndian)
                    .map_err(|e| {
                        OutputPluginError::OutputPluginFailed(format!(
                            "failed to write geometry as WKB: {}",
                            e
                        ))
                    })?;

                Ok(
                    out_bytes
                        .iter()
                        .map(|b| format!("{:02X?}", b))
                        .collect::<Vec<String>>()
                        .join("")
                )
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
