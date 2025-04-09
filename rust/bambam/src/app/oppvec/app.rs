use super::{OpportunityRecord, SourceFormat};
use crate::util::polygonal_rtree::PolygonalRTree;
use csv::Reader;
use itertools::Itertools;
use kdam::{term, tqdm, Bar, BarExt};
use rayon::prelude::*;
use routee_compass_core::{
    model::{
        map::{MapModelConfig, NearestSearchResult, SpatialIndex},
        network::Vertex,
        unit::{Distance, DistanceUnit},
    },
    util::fs::read_utils,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use wkt;

/// reads in opportunity data from some long-formatted opportunity dataset and aggregates
/// it to some vertex dataset
pub fn run(
    vertices_compass_filename: &String,
    opportunities_filename: &String,
    output_filename: &String,
    source_format: &SourceFormat,
    activity_categories: &[String],
) -> Result<(), String> {
    // load Compass Vertices, create spatial index
    let bar_builder = Bar::builder().desc("read vertices file");
    let vertices: Box<[Vertex]> =
        read_utils::from_csv(vertices_compass_filename, true, Some(bar_builder), None)
            .map_err(|e| format!("{}", e))?;
    let spatial_index = Arc::new(SpatialIndex::new_vertex_oriented(
        &vertices,
        Some((Distance::from(200.0), DistanceUnit::Meters)),
    ));

    // load opportunity data, build activity types lookup
    let opportunities: Vec<OppRow> =
        read_opportunity_rows_v2(opportunities_filename, source_format)?;
    // let acts_iter = tqdm!(
    //     opportunities.iter(),
    //     desc = "find all present unique categories",
    //     total = opportunities.len()
    // );
    // let activity_types_lookup = acts_iter
    //     .map(|(_, (_, cat))| cat.clone())
    //     .unique()
    //     .sorted()
    //     .enumerate()
    //     .map(|(i, c)| (c, i))
    //     .collect::<HashMap<_, _>>();
    // eprintln!();
    let activity_types_lookup = activity_categories
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect::<HashMap<_, _>>();

    // find a VertexId for each opportunity via nearest neighbors search tree
    let nearest_bar = Arc::new(Mutex::new(
        Bar::builder()
            .desc("attach opportunities to graph")
            .total(opportunities.len())
            .build()
            .unwrap(),
    ));
    let nearest_results = opportunities
        .into_par_iter()
        .flat_map(
            |OppRow {
                 geometry,
                 index,
                 category,
                 count,
             }| {
                if let Ok(mut bar) = nearest_bar.clone().lock() {
                    let _ = bar.update(1);
                }
                match spatial_index.clone().nearest_graph_id(&geometry) {
                    Ok(NearestSearchResult::NearestVertex(vertex_id)) => {
                        Some((vertex_id, (geometry, index, category)))
                    }
                    _ => None,
                }
            },
        )
        .collect_vec_list();
    eprintln!();

    // group opportunities by nearest vertex id
    let group_iter = tqdm!(
        nearest_results.into_iter(),
        desc = "group opportunities by vertex id"
    );
    let grouped = group_iter.flatten().into_group_map();
    eprintln!();

    // aggregate long-format data to wide-format using the activity type lookup to
    // increment values at vector indices.
    term::init(false);
    term::hide_cursor().map_err(|e| format!("progress bar error: {}", e))?;
    let result_iter = tqdm!(
        vertices.iter(),
        desc = "aggregate opportunities",
        total = vertices.len(),
        position = 0
    );

    let mut act_bars = activity_types_lookup
        .iter()
        .map(|(act, index)| {
            Bar::builder()
                .position(*index as u16 + 1)
                .desc(act)
                .build()
                .unwrap()
        })
        .collect_vec();
    let result: Vec<Vec<u64>> = result_iter
        .map(|v| match grouped.get(&v.vertex_id) {
            None => Ok(vec![0; activity_types_lookup.len()]),
            Some(opps) => {
                let mut out_row = vec![0; activity_types_lookup.len()];
                for (_, _, cat) in opps.iter() {
                    match activity_types_lookup.get(cat) {
                        None => {}
                        Some(out_index) => {
                            let _ = act_bars[*out_index].update(1);
                            out_row[*out_index] += 1;
                        }
                    }
                    // let out_index = activity_types_lookup.get(cat).ok_or_else(|| {
                    //     format!(
                    //         "internal error: missing category index for opportunity category '{}'",
                    //         cat
                    //     )
                    // })?;
                    // let _ = act_bars[*out_index].update(1);
                    // out_row[*out_index] = out_row[*out_index] + 1;
                }
                Ok(out_row)
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    eprintln!();
    for _ in act_bars.iter() {
        eprintln!();
    }
    term::show_cursor().map_err(|e| format!("progress bar error: {}", e))?;

    // write opportunity vectors
    let opportunities_compass_file = Path::new(output_filename);
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(opportunities_compass_file)
        .map_err(|e| {
            let opp_file_str = opportunities_compass_file.to_string_lossy();
            format!("failure opening output file {}: {}", opp_file_str, e)
        })?;
    let n_output_rows = result.len();
    let write_iter = tqdm!(
        result.into_iter().enumerate(),
        desc = "writing opportunities.csv",
        total = n_output_rows
    );
    for (idx, row) in write_iter {
        let serialized = row.into_iter().map(|v| format!("{}", v)).collect_vec();
        writer
            .write_record(&serialized)
            .map_err(|e| format!("failure writing CSV output row {}: {}", idx, e))?;
    }
    eprintln!();

    // // write activity types from this dataset to a file, preserving vector ordering
    // let opportunities_list_file = Path::new(output_filename).join("activity_types.json");
    // let act_types_list = activity_types_lookup
    //     .iter()
    //     .sorted_by_key(|(_, idx)| *idx)
    //     .map(|(c, _)| c.clone())
    //     .collect_vec();
    // let act_types_str = serde_json::to_string_pretty(&act_types_list)
    //     .map_err(|e| format!("failure JSON-encoding activity types by index: {}", e))?;
    // std::fs::write(&opportunities_list_file, act_types_str).map_err(|e| {
    //     format!(
    //         "failure writing {}: {}",
    //         &opportunities_list_file.to_string_lossy(),
    //         e
    //     )
    // })?;

    Ok(())
}

pub struct OppRow {
    pub geometry: geo::Point<f32>,
    pub index: usize,
    pub category: String,
    pub count: u64,
}

pub fn read_opportunity_rows_v2(
    opportunities_filename: &String,
    source_format: &SourceFormat,
) -> Result<Vec<OppRow>, String> {
    let mut opps_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(opportunities_filename)
        .map_err(|e| format!("failed to load {}: {}", opportunities_filename, e))?;
    let headers = build_header_lookup(&mut opps_reader)?;
    let bar = Arc::new(Mutex::new(
        Bar::builder()
            .desc("deserialize opportunities")
            .build()
            .map_err(|e| e.to_string())?,
    ));
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let outer_errors = errors.clone();
    let result = opps_reader
        .records()
        .enumerate()
        .collect_vec()
        .into_par_iter()
        .flat_map(|(idx, row)| {
            let inner_bar_clone = bar.clone();
            let mut inner_bar = match handle_failure(inner_bar_clone.lock(), errors.clone()) {
                Some(b) => b,
                None => return vec![],
            };
            inner_bar.update(1);
            let record = match handle_failure(row, errors.clone()) {
                Some(r) => r,
                None => return vec![],
            };
            let geometry_opt = match handle_failure(
                source_format.read_geometry(&record, &headers),
                errors.clone(),
            ) {
                Some(g_opt) => g_opt,
                None => return vec![],
            };
            let counts_by_category = match handle_failure(
                source_format.get_counts_by_category(&record, &headers),
                errors.clone(),
            ) {
                Some(cats) => cats,
                None => return vec![],
            };
            match geometry_opt {
                None => return vec![],
                Some(geometry) => {
                    return counts_by_category
                        .into_iter()
                        .map(|(act, cnt)| OppRow {
                            geometry: geometry.clone(),
                            index: idx,
                            category: act.clone(),
                            count: cnt,
                        })
                        .collect_vec()
                }
            }
        })
        .collect_vec_list()
        .into_iter()
        .flatten()
        .collect_vec();
    eprintln!();

    let final_errors = match outer_errors.lock() {
        Err(e) => return Err(e.to_string()),
        Ok(final_errors) => final_errors,
    };
    // let final_errors = errors.clone().lock();
    if final_errors.is_empty() {
        Ok(result)
    } else {
        Err(final_errors.iter().join(","))
    }
}

fn handle_failure<'a, T, E: ToString>(
    result: Result<T, E>,
    errors: Arc<Mutex<Vec<String>>>,
) -> Option<T> {
    match result.map_err(|e| e.to_string()) {
        Ok(t) => Some(t),
        Err(e) => {
            if let Ok(mut errs) = errors.clone().lock() {
                errs.push(e)
            }
            None
        }
    }
}

// pub fn read_opportunity_rows(
//     opportunities_filename: &String,
//     source_format: &SourceFormat,
//     category_filter: Option<&String>,
// ) -> Result<Vec<(geo::Point<f32>, (usize, String))>, String> {
//     // rust/bambam/src/app/oppvec/overture_categories.csv
//     // let overture_categories_filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//     //     .join("src")
//     //     .join("app")
//     //     .join("oppvec")
//     //     .join("overture_categories.csv");

//     // let mut cats_reader = csv::ReaderBuilder::new()
//     //     .has_headers(true)
//     //     .delimiter(b';')
//     //     .from_path(&overture_categories_filepath)
//     //     .map_err(|e| format!("failed to open overture_categories.csv file: {}", e))?;

//     // let parent_lookup = cats_reader
//     //     .into_records()
//     //     .map(|record| {
//     //         let r = record
//     //             .map_err(|e| format!("failure reading overture_categories.csv row: {}", e))?;
//     //         let category_code = r
//     //             .get(0)
//     //             .map(String::from)
//     //             .ok_or_else(|| String::from("row missing column 0"))?;
//     //         let taxonomy_str = r
//     //             .get(1)
//     //             .ok_or_else(|| String::from("row missing column 1"))?;
//     //         let taxonomy_vec = taxonomy_str
//     //             .replace("[", "")
//     //             .replace("]", "")
//     //             .split(",")
//     //             .map(String::from)
//     //             .collect_vec();
//     //         let parent = taxonomy_vec
//     //             .first()
//     //             .ok_or_else(|| format!("taxonomy for {} has no parent", category_code))?
//     //             .trim()
//     //             .to_string();
//     //         Ok((category_code, parent))
//     //     })
//     //     .collect::<Result<HashMap<_, _>, String>>()?;

//     let mut opps_reader = csv::ReaderBuilder::new()
//         .has_headers(true)
//         .from_path(opportunities_filename)
//         .map_err(|e| format!("failed to load {}: {}", opportunities_filename, e))?;
//     let headers = build_header_lookup(&mut opps_reader)?;
//     // let geom_idx = headers
//     //     .get(geometry_column)
//     //     .ok_or_else(|| String::from("internal error"))?;
//     let iter = tqdm!(
//         opps_reader.records().enumerate(),
//         desc = "deserialize opportunities"
//     );
//     let result = iter
//         .map(|(idx, row)| {
//             let record = row.map_err(|e| format!("failed to read row {}: {}", idx, e))?;
//             // let geometry_str = &record
//             //     .get(*geom_idx)
//             //     .ok_or_else(|| format!("row {} missing '{}' column", idx, geometry_column))?;
//             // let geometry: geo::Point<f32> = wkt::TryFromWkt::try_from_wkt_str(geometry_str)
//             //     .map_err(|e| format!("row {} has invalid Point geometry: {}", idx, e))?;
//             let geometry_opt = source_format.read_geometry(&record, &headers)?;
//             let category_opt = source_format.get_counts_by_category(&record, &headers)?;
//             match geometry_opt {
//                 None => Ok(None),
//                 Some(geometry) => {}
//             }

//             // match (geometry_opt, category_opt) {
//             //     (Some(geometry), Some(category)) => {
//             //         // let parent = parent_lookup.get(&category).ok_or_else(|| {
//             //         //     format!("missing parent lookup value for category '{}' where the cat_str was '{}'", category, cat_str)
//             //         // // })?;
//             //         match parent_lookup.get(&category) {
//             //             Some(parent) => {
//             //                 if accept_category_fn(parent) {
//             //                     Ok(Some((geometry, (idx, category))))
//             //                 } else {
//             //                     Ok(None)
//             //                 }
//             //             }
//             //             None => Ok(None),
//             //         }

//             //         // log::debug!("accepting row {} with category {}", idx, category);
//             //     }
//             //     _ => Ok(None),
//             // }
//         })
//         .collect::<Result<Vec<_>, String>>()?
//         .into_iter()
//         .flatten()
//         .collect_vec();
//     eprintln!();
//     Ok(result)
// }

pub fn build_header_lookup(reader: &mut Reader<File>) -> Result<HashMap<String, usize>, String> {
    // We nest this call in its own scope because of lifetimes.
    let headers = reader
        .headers()
        .map_err(|e| format!("failure retrieving headers: {}", e))?;
    let lookup: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(idx, col)| (String::from(col), idx))
        .collect::<HashMap<_, _>>();
    // for col in columns {
    //     if !lookup.contains_key(*col) {
    //         let header_str = lookup.keys().join(",");
    //         return Err(format!(
    //             "column '{}' not found in headers: [{}]",
    //             col, header_str
    //         ));
    //     }
    // }
    Ok(lookup)
}
