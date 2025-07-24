//! script to aggregate mep output rows to some overlay geometry dataset.
//! the number of output rows is not dependent on the size of the source geometry dataset,
//! instead based on the number of geometry rows with matches in the mep dataset.
//! only mep score and population data are aggregated at this time, via summation.
use super::OverlayOperation;
use crate::{app::overlay::OverlaySource, util::polygonal_rtree::PolygonalRTree as PrtBambam};
use bamsoda_core::model::identifier::Geoid;
use csv::StringRecord;
use geo::Geometry;
use itertools::Itertools;
use kdam::{tqdm, Bar, BarBuilder, BarExt};
use rayon::prelude::*;
use routee_compass_core::util::geo::PolygonalRTree;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, Read},
    path::Path,
    sync::{Arc, Mutex},
};
use wkt::{ToWkt, TryFromWkt};

// CSV rows as currently defined:
// grid_id,isochrone_10,isochrone_20,isochrone_30,isochrone_40,lat,lon,
// mep,mep_entertainment,mep_food,mep_healthcare,mep_jobs,mep_retail,mep_services,
// mode,opps_entertainment_10,opps_entertainment_20,opps_entertainment_30,opps_entertainment_40,opps_entertainment_total,
// opps_food_10,opps_food_20,opps_food_30,opps_food_40,opps_food_total,
// opps_healthcare_10,opps_healthcare_20,opps_healthcare_30,opps_healthcare_40,opps_healthcare_total,
// opps_jobs_10,opps_jobs_20,opps_jobs_30,opps_jobs_40,opps_jobs_total,
// opps_retail_10,opps_retail_20,opps_retail_30,opps_retail_40,opps_retail_total,opps_services_10,
// opps_services_20,opps_services_30,opps_services_40,opps_services_total,
// ram_mb,runtime_iter_opps,runtime_mep,runtime_opps,runtime_search
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MepRow {
    pub grid_id: String,
    pub lat: f64,
    pub lon: f64,
    pub mep: f64,
    pub mep_entertainment: f64,
    pub mep_food: f64,
    pub mep_healthcare: f64,
    pub mep_jobs: f64,
    pub mep_retail: f64,
    pub mep_services: f64,
    pub population: f64, // currently missing from rows
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutRow {
    pub geoid: Geoid,
    pub geometry: String,
    pub mep: f64,
    pub mep_entertainment: f64,
    pub mep_food: f64,
    pub mep_healthcare: f64,
    pub mep_jobs: f64,
    pub mep_retail: f64,
    pub mep_services: f64,
    pub population: f64, // currently missing from rows
}

impl OutRow {
    pub fn new(geoid: Geoid, geometry: &Geometry, rows: &[MepRow]) -> Self {
        let mut out_row = OutRow::empty(geoid.clone(), geometry);
        for row in rows.iter() {
            out_row.add(row);
        }
        out_row
    }

    /// sets up the OutRow with empty accumulators
    pub fn empty(geoid: Geoid, geometry: &Geometry) -> Self {
        OutRow {
            geoid,
            geometry: geometry.to_wkt().to_string(),
            mep: Default::default(),
            mep_entertainment: Default::default(),
            mep_food: Default::default(),
            mep_healthcare: Default::default(),
            mep_jobs: Default::default(),
            mep_retail: Default::default(),
            mep_services: Default::default(),
            population: Default::default(),
        }
    }

    pub fn add(&mut self, mep_row: &MepRow) {
        self.mep += mep_row.mep;
        self.mep_entertainment += mep_row.mep_entertainment;
        self.mep_food += mep_row.mep_food;
        self.mep_healthcare += mep_row.mep_healthcare;
        self.mep_jobs += mep_row.mep_jobs;
        self.mep_retail += mep_row.mep_retail;
        self.mep_services += mep_row.mep_services;
        self.population += mep_row.population;
    }
}

/// runs a
pub fn run2(
    mep_filepath: &str,
    output_filepath: &str,
    overlay_source: &OverlaySource,
    how: &OverlayOperation,
    chunksize: usize,
) -> Result<(), String> {
    // fail early if IO error from read/write destinations
    let mep_result_file =
        File::open(mep_filepath).map_err(|e| format!("error reading '{mep_filepath}': {e}"))?;
    let mut output_writer = csv::Writer::from_path(output_filepath)
        .map_err(|e| format!("failure opening output file '{}': {}", output_filepath, e))?;

    // read overlay dataset
    let overlay_data = overlay_source.build()?;
    let overlay_lookup = overlay_data
        .iter()
        .map(|(geom, geoid)| (geoid.clone(), geom.clone()))
        .collect::<HashMap<_, _>>();
    let overlay: Arc<PolygonalRTree<f64, Geoid>> = Arc::new(PolygonalRTree::new(overlay_data)?);

    // Read chunks of CSV rows at a time. the mep output can be very large on the order of 10s of GBs.
    let mut bar = BarBuilder::default()
        .desc("chunking mep data rows")
        .position(0)
        .build()?;
    let mut chunk = String::new();
    let mut chunks = 0;
    let mut lines_read = 0;
    let mut buf_reader = std::io::BufReader::new(mep_result_file);
    let mut grouped: HashMap<Geoid, (Geometry, Vec<MepRow>)> = HashMap::new();
    loop {
        // Read a chunk of lines into `chunk`
        chunk.clear();
        chunks += 1;
        let _ = bar.update(1);
        let mut lines = buf_reader.by_ref().lines().take(chunksize);
        let mut any = false;
        while let Some(line) = lines.next() {
            let line = line.map_err(|e| format!("error reading line: {e}"))?;
            chunk.push_str(&line);
            chunk.push('\n');
            lines_read += 1;
            any = true;
        }
        if !any {
            // done reading from the buffer, all rows processed
            break;
        }

        // read the next chunk, process it, and append the grouped collection
        let mut reader = csv::Reader::from_reader(chunk.as_bytes());
        let rows = reader
            .deserialize()
            .collect::<Result<Vec<MepRow>, _>>()
            .map_err(|e| format!("failure deserializing chunk {chunks}: {e}"))?;
        let tagged_rows: Vec<(Geoid, MepRow)> =
            match_chunk(rows, output_filepath, overlay.clone())?;
        for (geoid, row) in tagged_rows.into_iter() {
            match grouped.get_mut(&geoid) {
                Some((_, v)) => v.push(row),
                None => {
                    let geometry = overlay_lookup.get(&geoid).ok_or_else(|| {
                        format!("internal error, lookup missing geometry entry for geoid '{geoid}'")
                    })?;
                    let _ = grouped.insert(geoid, (geometry.clone(), vec![row]));
                }
            }
        }
    }

    // aggregate results
    let result = grouped
        .into_iter()
        .map(|(geoid, (geometry, mep_rows))| OutRow::new(geoid, &geometry, &mep_rows))
        .collect_vec();

    for row in result.into_iter() {
        output_writer
            .serialize(row)
            .map_err(|e| format!("failure writing row to output: {e}"))?;
    }

    println!("written to {output_filepath}");
    Ok(())
}

fn match_chunk(
    rows: Vec<MepRow>,
    output_filename: &str,
    overlay: Arc<PolygonalRTree<f64, Geoid>>,
) -> Result<Vec<(Geoid, MepRow)>, String> {
    let bar = Arc::new(Mutex::new(
        BarBuilder::default()
            .position(1)
            .desc("spatial lookup")
            .total(rows.len())
            .build()?,
    ));
    rows.into_par_iter()
        .flat_map(|row| {
            if let Ok(mut b) = bar.clone().lock() {
                let _ = b.update(1);
            }
            let point = geo::Geometry::Point(geo::Point::new(row.lon, row.lat));
            let intersection_result = overlay.intersection(&point);
            let found = match intersection_result {
                Err(e) => return vec![Err(e)],
                Ok(found) => found.collect_vec(),
            };
            match found[..] {
                [] => vec![],
                [single] => vec![Ok((single.data.clone(), row))],
                _ => {
                    let found_geoids = found.iter().map(|r| r.data.to_string()).join(", ");
                    vec![Err(format!(
                        "point {} unexpectedly found multiple geoids: [{}]",
                        point.to_wkt().to_string(),
                        found_geoids
                    ))]
                }
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

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
                .ok_or_else(|| format!("row {} missing geometry index", row_idx))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            Ok((geometry, r.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bambam_rtree = Arc::new(PrtBambam::new(bambam_geometries)?);

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
        .ok_or_else(|| format!("overlay file missing {} column", geometry_column))?;
    let overlay_id_idx = overlay_headers
        .get(id_column.as_str())
        .ok_or_else(|| format!("overlay file missing {} column", id_column))?;

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
                .ok_or_else(|| format!("row {} missing geometry index", row_idx))?;
            let geometry: Geometry =
                Geometry::try_from_wkt_str(geometry_str).map_err(|e| e.to_string())?;
            let id = r
                .get(*overlay_id_idx)
                .ok_or_else(|| format!("row {} missing id index", row_idx))?
                .to_string();

            match how {
                OverlayOperation::Intersection => {
                    for node in bambam_rtree.intersection(&geometry)? {
                        let mut out: StringRecord = StringRecord::new();
                        for (j, sr) in node.data.iter().enumerate() {
                            if j == *bambam_geometry_lookup {
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
