use std::{collections::HashMap, path::Path};

use bamcensus_core::model::identifier::Geoid;
use geo::Geometry;
use routee_compass_core::util::geo::PolygonalRTree;
use serde::{Deserialize, Serialize};
use wkt::TryFromWkt;

/// source of overlay geometry dataset
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum OverlaySource {
    /// reads overlay geometries from a CSV file that contains geometry and id columns
    Csv {
        file: String,
        geometry_column: String,
        id_column: String,
    },
    /// reads overlay geometries from a shapefile with an id field
    Shapefile { file: String, id_field: String },
    /// uses bamcensus-tiger to retrieve the geometries associated with the given geoids from the web
    TigerLines { geoids: Vec<Geoid> },
}

impl OverlaySource {
    pub fn build(&self) -> Result<Vec<(Geometry, Geoid)>, String> {
        match self {
            OverlaySource::Csv {
                file,
                geometry_column,
                id_column,
            } => read_overlay_csv(file, geometry_column, id_column),
            OverlaySource::Shapefile { file, id_field } => read_overlay_shapefile(file, id_field),
            OverlaySource::TigerLines { geoids } => {
                todo!("not yet implemented, requires improvements to bamcensus")
            }
        }
    }
}

/// reads geometries and Geoids from a shapefile source
fn read_overlay_shapefile(
    overlay_filepath: &str,
    id_field: &str,
) -> Result<Vec<(Geometry, Geoid)>, String> {
    let rows = shapefile::read(overlay_filepath)
        .map_err(|e| format!("failed reading '{overlay_filepath}': {e}"))?;

    let mut processed = vec![];
    for (idx, (shape, record)) in rows.into_iter().enumerate() {
        let geometry = match shape {
            shapefile::Shape::Polygon(generic_polygon) => {
                let mp: geo::MultiPolygon<f64> = generic_polygon.try_into().map_err(|e| {
                    format!("failed to convert shapefile polygon at row {idx}: {e}")
                })?;
                geo::Geometry::MultiPolygon(mp)
            }
            shapefile::Shape::PolygonM(generic_polygon) => {
                let mp: geo::MultiPolygon<f64> = generic_polygon.try_into().map_err(|e| {
                    format!("failed to convert shapefile polygon at row {idx}: {e}")
                })?;
                geo::Geometry::MultiPolygon(mp)
            }
            _ => {
                return Err(format!(
                    "unexpected shape type {} found at row {}, must be polygonal",
                    shape.shapetype(),
                    idx
                ))
            }
        };
        let field = record
            .get(id_field)
            .ok_or_else(|| format!("field {id_field} missing from shapefile record"))?;
        let geoid = match field {
            shapefile::dbase::FieldValue::Character(Some(s)) => Geoid::try_from(s.as_str()),
            _ => Err(format!(
                "field '{}' has unexpected field type '{}'",
                id_field,
                field.field_type()
            )),
        }?;
        processed.push((geometry, geoid));
    }
    Ok(processed)
}

/// reads geometries and Geoids from a CSV source
fn read_overlay_csv(
    overlay_filepath: &str,
    geometry_column: &str,
    id_column: &str,
) -> Result<Vec<(Geometry, Geoid)>, String> {
    // read in overlay geometries file
    let overlay_path = Path::new(overlay_filepath);
    let mut overlay_reader = csv::Reader::from_path(overlay_path).map_err(|e| e.to_string())?;
    let overlay_header_record = overlay_reader.headers().map_err(|e| e.to_string())?.clone();
    let overlay_headers = overlay_header_record
        .into_iter()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect::<HashMap<_, _>>();
    let overlay_geom_idx = overlay_headers
        .get(geometry_column)
        .ok_or_else(|| format!("overlay file missing {geometry_column} column"))?;
    let overlay_id_idx = overlay_headers
        .get(id_column)
        .ok_or_else(|| format!("overlay file missing {id_column} column"))?;

    let overlay_data = overlay_reader
        .records()
        .enumerate()
        .map(|(idx, r)| {
            let row = r.map_err(|e| e.to_string())?;
            let geometry_str = row
                .get(*overlay_geom_idx)
                .ok_or_else(|| format!("row {idx} missing geometry index"))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            let id_str = row
                .get(*overlay_id_idx)
                .ok_or_else(|| format!("row {idx} missing id index"))?
                .to_string();
            let geoid = Geoid::try_from(id_str.as_str())?;

            match geometry {
                Geometry::Point(_) => Err(format!(
                    "unexpected Point geometry type for row {idx} with id {geoid}"
                )),
                Geometry::Line(_) => Err(format!(
                    "unexpected Line geometry type for row {idx} with id {geoid}"
                )),
                Geometry::LineString(_) => Err(format!(
                    "unexpected LineString geometry type for row {idx} with id {geoid}"
                )),
                Geometry::Polygon(_) => Ok(()),
                Geometry::MultiPoint(_) => Err(format!(
                    "unexpected MultiPoint geometry type for row {idx} with id {geoid}"
                )),
                Geometry::MultiLineString(_) => Err(format!(
                    "unexpected MultiLineString geometry type for row {idx} with id {geoid}"
                )),
                Geometry::MultiPolygon(_) => Ok(()),
                Geometry::GeometryCollection(_) => Err(format!(
                    "unexpected GeometryCollection geometry type for row {idx} with id {geoid}"
                )),
                Geometry::Rect(_) => Err(format!(
                    "unexpected Rect geometry type for row {idx} with id {geoid}"
                )),
                Geometry::Triangle(_) => Err(format!(
                    "unexpected Triangle geometry type for row {idx} with id {geoid}"
                )),
            }?;

            Ok((geometry, geoid))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(overlay_data)
}
