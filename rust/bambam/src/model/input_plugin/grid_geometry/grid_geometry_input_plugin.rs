use geo::Polygon;
use h3o::CellIndex;
use routee_compass::app::search::SearchApp;
use routee_compass::plugin::input::{InputField, InputPlugin, InputPluginError};
use routee_compass_core::config::ConfigJsonExtensions;
use serde_json;
use std::sync::Arc;
use wkb;

pub struct GridGeometryInputPlugin;

impl InputPlugin for GridGeometryInputPlugin {
    fn process(
        &self,
        input: &mut serde_json::Value,
        search_app: Arc<SearchApp>,
    ) -> Result<(), InputPluginError> {
        // Retrieve Cell string
        let row_grid_id = input.get_config_string(&super::GRID_ID, &"").map_err(|e| {
            InputPluginError::MissingExpectedQueryField(InputField::Custom(String::from("grid_id")))
        })?;

        // Map to correct type
        let grid_id_value: u64 = u64::from_str_radix(&row_grid_id, 16).map_err(|e| {
            InputPluginError::InputPluginFailed(format!(
                "Invalid grid value. {row_grid_id} cannot be interpreted as u64"
            ))
        })?;
        let h3_cell: CellIndex = CellIndex::try_from(grid_id_value).map_err(|e| {
            InputPluginError::InputPluginFailed(
                "Failed to build CellIndex struct from u64".to_string(),
            )
        })?;

        // Convert to WKB
        let mut out_bytes: Vec<u8> = vec![];
        wkb::writer::write_polygon(
            &mut out_bytes,
            &Polygon::from(h3_cell),
            &wkb::writer::WriteOptions {
                endianness: wkb::Endianness::BigEndian,
            },
        );

        // Write to query
        input["geometry"] = serde_json::Value::String(
            out_bytes
                .iter()
                .map(|b| format!("{b:02X?}"))
                .collect::<Vec<String>>()
                .join(""),
        );

        Ok(())
    }
}
