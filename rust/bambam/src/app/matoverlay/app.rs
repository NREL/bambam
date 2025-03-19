use super::Overlay;
use crate::util::polygonal_rtree::PolygonalRTree;
use csv::StringRecord;
use geo::Geometry;
use itertools::Itertools;
use kdam::tqdm;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use wkt::TryFromWkt;

pub fn run(
    mep_matrix_filename: &String,
    overlay_filename: &String,
    how: &Overlay,
    geometry_column: &String,
    id_column: &String,
) -> Result<(), String> {
    let mat_path = Path::new(mep_matrix_filename);
    let mut mat_reader = csv::Reader::from_path(mat_path).map_err(|e| e.to_string())?;
    let mat_header_record = mat_reader.headers().map_err(|e| e.to_string())?.clone();
    let mat_header_lookup = mat_header_record
        .iter()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect::<HashMap<_, _>>();
    let mat_geom_idx = mat_header_lookup
        .get("geometry")
        .ok_or_else(|| String::from("overlay file missing `geometry` column"))?;
    let mat_vec = mat_reader
        .records()
        .enumerate()
        .map(|(row_idx, row)| {
            let r = row.map_err(|e| e.to_string())?;
            let geometry_str = r
                .get(*mat_geom_idx)
                .ok_or_else(|| format!("row {} missing geometry index", row_idx))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            Ok((geometry, r.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mat_rtree = PolygonalRTree::new(mat_vec)?;
    let overlay_path = Path::new(overlay_filename);
    let mut overlay_reader = csv::Reader::from_path(overlay_path).map_err(|e| e.to_string())?;
    let overlay_header_record = overlay_reader.headers().map_err(|e| e.to_string())?.clone();
    let overlay_headers = overlay_header_record
        .into_iter()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect::<HashMap<_, _>>();
    let overlay_geom_idx = overlay_headers
        .get(geometry_column.as_str())
        .ok_or_else(|| format!("overlay file missing {} column", geometry_column))?;
    let overlay_id_idx = overlay_headers
        .get(id_column.as_str())
        .ok_or_else(|| format!("overlay file missing {} column", id_column))?;

    let out_headers = mat_header_record.iter().chain(["join_id"]).join(",");
    println!("{}", out_headers);

    for (row_idx, row) in tqdm!(overlay_reader.records().enumerate(), desc = "process") {
        let r = row.map_err(|e| e.to_string())?;
        let geometry_str = r
            .get(*overlay_geom_idx)
            .ok_or_else(|| format!("row {} missing geometry index", row_idx))?;
        let geometry: Geometry =
            Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
        let id = r
            .get(*overlay_id_idx)
            .ok_or_else(|| format!("row {} missing id index", row_idx))?
            .to_string();

        match how {
            Overlay::Intersection => {
                for node in mat_rtree.intersection(&geometry)? {
                    let mut out: StringRecord = StringRecord::new();
                    for (j, sr) in node.data.iter().enumerate() {
                        if j == *mat_geom_idx {
                            let enquoted = format!("\"{}\"", sr);
                            out.push_field(enquoted.as_str());
                        } else {
                            out.push_field(sr);
                        }
                    }
                    out.push_field(&id);
                    println!("{}", out.into_iter().join(","));
                }
            }
        }
    }
    Ok(())
}
