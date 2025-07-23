use super::OverlayOperation;
use crate::util::polygonal_rtree::PolygonalRTree;
use csv::StringRecord;
use geo::Geometry;
use itertools::Itertools;
use kdam::{tqdm, Bar};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};
use wkt::TryFromWkt;

/// aggregate a bambam output to some other geospatial dataset via some overlay operation.
///
/// # Arguments
/// * `bambam_output_filepath` - an output CSV file from a bambam run
/// * `overlay_filepath` - a file containing the overlay geometry dataset
/// * `how` - an overlay method, a map algebra
/// * `geometry_column` - column in overlay file containing a WKT geometry
/// * `id_column` - column in overlay file containing an identifier
///
/// # Result
///
///
pub fn run(
    bambam_output_filepath: &String,
    overlay_filepath: &String,
    output_filename: &String,
    how: &OverlayOperation,
    geometry_column: &String,
    id_column: &String,
) -> Result<(), String> {
    // read in bambam outputs into a spatial index
    let bambam_path = Path::new(bambam_output_filepath);
    let mut bambam_reader = csv::Reader::from_path(bambam_path).map_err(|e| e.to_string())?;
    let bambam_header_record = bambam_reader.headers().map_err(|e| e.to_string())?.clone();
    let bambam_header_lookup = bambam_header_record
        .iter()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect::<HashMap<_, _>>();
    let bambam_geometry_lookup = bambam_header_lookup
        .get("geometry")
        .ok_or_else(|| String::from("overlay file missing `geometry` column"))?;
    let bambam_geometries = bambam_reader
        .records()
        .enumerate()
        .map(|(row_idx, row)| {
            let r = row.map_err(|e| e.to_string())?;
            let geometry_str = r
                .get(*bambam_geometry_lookup)
                .ok_or_else(|| format!("row {row_idx} missing geometry index"))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            Ok((geometry, r.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bambam_rtree = Arc::new(PolygonalRTree::new(bambam_geometries)?);

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
        .get(geometry_column.as_str())
        .ok_or_else(|| format!("overlay file missing {geometry_column} column"))?;
    let overlay_id_idx = overlay_headers
        .get(id_column.as_str())
        .ok_or_else(|| format!("overlay file missing {id_column} column"))?;

    // TODO!
    //  this just writes the aggregated overlay dataset to stdout
    //  let's write to a file location
    //  let's parallelize the intersection operation
    let overlay_dataset = overlay_reader.records().enumerate().collect_vec();
    let overlay_bar = Arc::new(Mutex::new(
        Bar::builder()
            .desc("overlay dataset")
            .total(overlay_dataset.len())
            .build()
            .map_err(|e| e.to_string())?,
    ));

    let result: std::collections::LinkedList<Vec<Result<_, String>>> = overlay_dataset
        .into_par_iter()
        .map(|(row_idx, row)| {
            let r = row.map_err(|e| e.to_string())?;

            let geometry_str = r
                .get(*overlay_geom_idx)
                .ok_or_else(|| format!("row {row_idx} missing geometry index"))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            let id = r
                .get(*overlay_id_idx)
                .ok_or_else(|| format!("row {row_idx} missing id index"))?
                .to_string();

            match how {
                OverlayOperation::Intersection => {
                    for node in bambam_rtree.intersection(&geometry)? {
                        let mut out: StringRecord = StringRecord::new();
                        for (j, sr) in node.data.iter().enumerate() {
                            if j == *bambam_geometry_lookup {
                                let enquoted = format!("\"{sr}\"");
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
            Ok(todo!())
        })
        .collect_vec_list();

    todo!("unsure what is below, does it aggregate the rows?");

    let output_filepath = Path::new(output_filename);

    // graveyard:

    // let out_headers = bambam_header_record.iter().chain(["join_id"]).join(",");
    // println!("{}", out_headers);

    // for (row_idx, row) in tqdm!(overlay_reader.records().enumerate(), desc = "process") {
    //     let r = row.map_err(|e| e.to_string())?;
    //     let geometry_str = r
    //         .get(*overlay_geom_idx)
    //         .ok_or_else(|| format!("row {} missing geometry index", row_idx))?;
    //     let geometry: Geometry =
    //         Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
    //     let id = r
    //         .get(*overlay_id_idx)
    //         .ok_or_else(|| format!("row {} missing id index", row_idx))?
    //         .to_string();

    //     match how {
    //         OverlayOperation::Intersection => {
    //             for node in bambam_rtree.intersection(&geometry)? {
    //                 let mut out: StringRecord = StringRecord::new();
    //                 for (j, sr) in node.data.iter().enumerate() {
    //                     if j == *bambam_geometry_lookup {
    //                         let enquoted = format!("\"{}\"", sr);
    //                         out.push_field(enquoted.as_str());
    //                     } else {
    //                         out.push_field(sr);
    //                     }
    //                 }
    //                 out.push_field(&id);
    //                 println!("{}", out.into_iter().join(","));
    //             }
    //         }
    //     }
    // }
    Ok(())
}
