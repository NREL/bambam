use geo::Geometry;
use routee_compass::plugin::{output::OutputPluginError, PluginError};
use serde::{Deserialize, Serialize};
use wkb::*;
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
                let bytes = geom_to_wkb(geometry).map_err(|e| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "failed to generate wkb for geometry {:?} - {:?}",
                        geometry, e
                    ))
                })?;
                let wkb_str = bytes
                    .iter()
                    .map(|b| format!("{:02X?}", b))
                    .collect::<Vec<String>>()
                    .join("");
                Ok(wkb_str)
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
