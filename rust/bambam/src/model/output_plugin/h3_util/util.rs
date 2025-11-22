use geo::{Geometry, LineString, Polygon};
use h3o::CellIndex;
use jsonpath_rust::JsonPath;
use routee_compass::plugin::output::OutputPluginError;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::model::output_plugin::h3_util::{BoundaryGeometryFormat, DotDelimitedPath};

#[derive(Debug, Clone)]
pub enum H3Util {
    /// reads an h3 identifier from some path in the output JSON and uses
    /// h3 cellToBoundary to write the h3
    /// see [[https://h3geo.org/docs/api/indexing#celltoboundary]].
    H3BoundaryToGeometry {
        from: DotDelimitedPath,
        to: DotDelimitedPath,
        format: BoundaryGeometryFormat,
    },
    /// copies an h3 identifier from some JSONPath to another JSONPath,
    /// converting it to the declared parent resolution.
    H3ToParent {
        from: DotDelimitedPath,
        to: DotDelimitedPath,
        resolution: u8,
    },
}

impl H3Util {
    pub fn apply(&self, output: &mut Value) -> Result<(), OutputPluginError> {
        match self {
            H3Util::H3BoundaryToGeometry { from, to, format } => {
                let from_jsonpath = from.as_jsonpath();

                let hex_idx = get_hex(output, &from_jsonpath).map_err(|e| {
                    let msg = format!("while running h3_boundary_to_geometry, {e}");
                    OutputPluginError::OutputPluginFailed(msg)
                })?;

                // create boundary JSON
                let boundary: LineString = hex_idx.boundary().into();
                let polygon = Polygon::new(boundary, vec![]);
                let out_value = format.serialize(&polygon).map_err(|e| {
                    let msg = format!("while running h3_boundary_to_geometry, {e}");
                    OutputPluginError::OutputPluginFailed(msg)
                })?;

                // write to output JSON
                let to_pointer = to.as_json_pointer();
                match output.pointer_mut(&to_pointer) {
                    // todo: does this fail if we haven't put something at the leaf node yet?
                    Some(leaf) => {
                        *leaf = out_value;
                        Ok(())
                    }
                    None => {
                        let msg = format!("while running h3_boundary_to_geometry, failed to find output location '{to_pointer}'");
                        Err(OutputPluginError::OutputPluginFailed(msg))
                    }
                }?;

                Ok(())
            }
            H3Util::H3ToParent {
                from,
                to,
                resolution,
            } => todo!(),
        }
    }
}

/// helper function to get a single value from a JSON value at some JSONPath
fn get_single_value<T: DeserializeOwned>(output: &Value, json_path: &str) -> Result<T, String> {
    let found_values = output
        .query(json_path)
        .map_err(|e| format!("failed to find value at '{json_path}': {e}"))?;
    let found_value: T = match found_values[..] {
        [from_value] => serde_json::from_value(from_value.clone()).map_err(|e| e.to_string()),
        _ => Err(format!(
            "invalid path, found more than one value at '{json_path}'"
        )),
    }?;
    Ok(found_value)
}

fn get_hex(output: &Value, json_path: &str) -> Result<CellIndex, String> {
    let hex_str: String =
        get_single_value(output, json_path).map_err(|e| format!("while getting h3 hex, {e}"))?;
    let hex_idx = hex_str
        .parse::<CellIndex>()
        .map_err(|e| format!("while parsing '{hex_str}' into h3 hex, {e}"))?;
    Ok(hex_idx)
}
