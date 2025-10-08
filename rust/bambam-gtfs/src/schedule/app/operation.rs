//! GTFS archive pre-processing scripts for bambam-gtfs transit modeling.
//! see [https://github.com/MobilityData/mobility-database-catalogs] for
//! information on the mobility database catalog listing.
use crate::schedule::distance_calculation_policy::DistanceCalculationPolicy;
use crate::schedule::schedule_error::ScheduleError;
use crate::schedule::{bundle_ops, GtfsProvider, GtfsSummary, MissingStopLocationPolicy};
use chrono::NaiveDate;
use clap::{value_parser, Subcommand};
use geo::{Coord, LineString};
use gtfs_structures::Gtfs;
use itertools::Itertools;
use kdam::Bar;
use rayon::prelude::*;
use routee_compass_core::model::map::SpatialIndex;
use routee_compass_core::model::network::Vertex;
use routee_compass_core::util::fs::read_utils;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashSet, fs::File, io::Write, path::Path, time::Duration};
use uom::si::f64::Length;
use wkt::ToWkt;

#[derive(Debug, Clone, Serialize, Deserialize, Subcommand)]
pub enum GtfsOperation {
    /// summarize attributes for the downloaded GTFS archives
    Summary {
        /// file containing a list of GTFS arcives
        #[arg(long, default_value_t=String::from("2024-08-13-mobilitydataacatalog.csv"))]
        manifest_file: String,
        /// country code to filter from list, defaults to US-based transit options
        #[arg(long, default_value_t = String::from("US"))]
        country_code: String,
        /// data type to filter from list
        #[arg(long, default_value_t = String::from("gtfs"))]
        data_type: String,
    },
    /// download all WKT shapes data from the GTFS archives
    Shapes {
        #[arg(long, default_value_t=String::from("2024-08-13-mobilitydataacatalog.csv"))]
        manifest_file: String,
        /// country code to filter from list, defaults to US-based transit options
        #[arg(long, default_value_t = String::from("US"))]
        country_code: String,
        /// data type to filter from list
        #[arg(long, default_value_t = String::from("gtfs"))]
        data_type: String,
    },
    /// download all of the GTFS archives
    Download {
        #[arg(long, default_value_t = 1)]
        parallelism: usize,
        /// country code to filter from list, defaults to US-based transit options
        #[arg(long, default_value_t = String::from("US"))]
        country_code: String,
        /// data type to filter from list
        #[arg(long, default_value_t = String::from("gtfs"))]
        data_type: String,
        #[arg(long, default_value_t=String::from("2024-08-13-mobilitydataacatalog.csv"))]
        manifest_file: String,
    },
    /// Process bundle into EdgeLists
    PreprocessBundle {
        /// a single GTFS archive or a directory of GTFS archives
        #[arg(long)]
        input: String,
        /// in this case of a single input file, this sets the edge list id for that input.
        /// for a directory input, sets the starting edge list id.
        #[arg(long)]
        starting_edge_list_id: usize,

        #[arg(long, default_value_t = 1)]
        parallelism: usize,

        #[arg(long)]
        output_directory: String,

        #[arg(long)]
        vertices_compass_filename: String,

        #[arg(long, value_parser = value_parser!(NaiveDate))]
        start_date: NaiveDate,

        #[arg(long, value_parser = value_parser!(NaiveDate))]
        end_date: NaiveDate,

        #[arg(long, default_value_t = 325.)]
        vertex_match_tolerance: f64,

        #[arg(value_enum, default_value_t=MissingStopLocationPolicy::Fail)]
        missing_stop_location_policy: MissingStopLocationPolicy,

        #[arg(value_enum, default_value_t=DistanceCalculationPolicy::Haversine)]
        distance_calculation_policy: DistanceCalculationPolicy,

        #[arg(long, default_value_t = true)]
        overwrite: bool,
    },
}

impl GtfsOperation {
    pub fn run(&self) {
        match self {
            GtfsOperation::Summary {
                manifest_file,
                data_type,
                country_code,
            } => {
                let rows = manifest_into_rows(manifest_file, Some(country_code), Some(data_type))
                    .expect("failed reading manifest");
                summarize(&rows)
            }
            GtfsOperation::Shapes {
                manifest_file,
                country_code,
                data_type,
            } => {
                let rows = manifest_into_rows(manifest_file, Some(country_code), Some(data_type))
                    .expect("failed reading manifest");
                shapes(&rows)
            }
            GtfsOperation::Download {
                manifest_file,
                parallelism,
                data_type,
                country_code,
            } => {
                let rows = manifest_into_rows(manifest_file, Some(country_code), Some(data_type))
                    .expect("failed reading manifest");
                download(&rows, *parallelism)
            }
            GtfsOperation::PreprocessBundle {
                input,
                starting_edge_list_id,
                vertices_compass_filename,
                start_date,
                end_date,
                vertex_match_tolerance,
                missing_stop_location_policy,
                distance_calculation_policy,
                output_directory,
                overwrite,
                parallelism,
            } => {
                let spatial_index = load_vertices_and_create_spatial_index(
                    vertices_compass_filename,
                    *vertex_match_tolerance,
                )
                .expect("failed reading vertices and building spatial index");
                let input_path = Path::new(input);
                if input_path.is_dir() {
                    bundle_ops::process_bundles(
                        input_path,
                        starting_edge_list_id,
                        start_date,
                        end_date,
                        spatial_index,
                        missing_stop_location_policy,
                        distance_calculation_policy,
                        Path::new(output_directory),
                        *overwrite,
                        *parallelism,
                    )
                    .unwrap_or_else(|_| {
                        panic!("failed running GTFS processing operation for directory {input}")
                    })
                } else {
                    bundle_ops::process_bundle(
                        input,
                        starting_edge_list_id,
                        start_date,
                        end_date,
                        spatial_index,
                        missing_stop_location_policy,
                        distance_calculation_policy,
                        Path::new(output_directory),
                        *overwrite,
                    )
                    .unwrap_or_else(|_| {
                        panic!("failed running GTFS processing operation for input {input}")
                    })
                }
            }
        }
    }
}

fn load_vertices_and_create_spatial_index(
    vertices_compass_filename: &str,
    tolerance_meters: f64,
) -> Result<Arc<SpatialIndex>, ScheduleError> {
    // load Compass Vertices, create spatial index
    let bar_builder = Bar::builder().desc("read vertices file");
    let vertices: Box<[Vertex]> = read_utils::from_csv(
        &Path::new(vertices_compass_filename),
        true,
        Some(bar_builder),
        None,
    )
    .map_err(|e| ScheduleError::FailedToCreateVertexIndexError(format!("{e}")))?;
    let tol: Length = uom::si::f64::Length::new::<uom::si::length::meter>(tolerance_meters);
    Ok(Arc::new(SpatialIndex::new_vertex_oriented(
        &vertices,
        Some(tol),
    )))
}

/// reads rows from a GTFS manifest in the format of Mobility Data Catalog
/// see [https://github.com/MobilityData/mobility-database-catalogs].
///
/// # Arguments
///
/// * `country_code` - optional country to filter by
/// * `data_type` - optional data type to filter by
fn manifest_into_rows(
    manifest_file: &str,
    country_code: Option<&str>,
    data_type: Option<&str>,
) -> Result<Vec<GtfsProvider>, ScheduleError> {
    let path_buf = PathBuf::from(manifest_file);
    let reader = csv::ReaderBuilder::new()
        .from_path(path_buf.as_path())
        .map_err(|e| {
            let filename = path_buf.to_str().unwrap_or_default();
            ScheduleError::OtherError(format!("failure reading '{filename}': {e}"))
        })?;
    let rows = reader
        .into_deserialize::<GtfsProvider>()
        .map(|r| {
            r.map_err(|e| {
                ScheduleError::OtherError(format!("failure reading GTFS manifest row: {e}"))
            })
        })
        .collect::<Result<Vec<GtfsProvider>, ScheduleError>>()?;
    let us_rows: Vec<GtfsProvider> = rows
        .into_iter()
        .filter(|r| match (country_code, data_type) {
            (None, None) => true,
            (None, Some(dt)) => r.data_type.as_str() == dt,
            (Some(cc), None) => r.country_code.as_str() == cc,
            (Some(cc), Some(dt)) => r.country_code.as_str() == cc && r.data_type.as_str() == dt,
        })
        .collect_vec();

    Ok(us_rows)
}

fn summarize(rows: &Vec<GtfsProvider>) {
    let results = rows
        .par_iter()
        .map(|record| match &record.url {
            None => Ok((record, GtfsSummary::default())),
            Some(url) => match Gtfs::new(url) {
                Err(e) => Ok((record, GtfsSummary::error(format!("gtfs error: {e}")))),
                Ok(gtfs) => {
                    let n_trips = gtfs.trips.len();
                    let n_shapes = gtfs.shapes.len();
                    let mut n_legs = 0;
                    let mut n_unique_legs = 0;
                    let mut sum = 0;
                    for (_, trip) in gtfs.trips {
                        let mut leg_ods: HashSet<(&String, &String)> = HashSet::new();
                        for pair in trip.stop_times.windows(2) {
                            leg_ods.insert((&pair[0].stop.id, &pair[1].stop.id));
                        }
                        let trip_legs = (trip.stop_times.len() - 1).max(0); // stop_times are vertices, we want edges
                        n_legs += trip_legs;
                        n_unique_legs += leg_ods.len();

                        if let Some(shape_id) = trip.shape_id {
                            if gtfs.shapes.contains_key(&shape_id) {
                                sum += 1;
                            }
                        }
                    }
                    let coverage = sum as f64 / n_trips as f64;
                    let result = GtfsSummary {
                        message: String::from("success"),
                        coverage,
                        trips: n_trips,
                        shapes: n_shapes,
                        legs: n_legs,
                        unique_legs: n_unique_legs,
                    };
                    // println!("{}", record.provider);

                    Ok((record, result))
                }
            },
        })
        .collect::<Result<Vec<_>, String>>()
        .unwrap();

    println!("finished, with {} result rows", results.len());
    println!(
        "{} rows have active GTFS Agencies",
        results
            .iter()
            .filter(|r| r.1.message != *"inactive")
            .collect_vec()
            .len()
    );

    let mut out = File::create_new("gtfs_summaries.csv").unwrap();
    writeln!(
        out,
        "provider,url,message,coverage,trips,shapes,legs,unique_legs"
    )
    .unwrap();

    for (record, summary) in results {
        writeln!(out, "{record},{summary}").unwrap();
    }
}

/// todo: response should be Result so we can capture errors and report
/// at the end.
fn shapes(rows: &Vec<GtfsProvider>) {
    let results = rows
        .par_iter()
        .flat_map(|record| match &record.url {
            None => vec![],
            Some(url) => match Gtfs::new(url) {
                Err(_) => vec![],
                Ok(gtfs) => {
                    let rows = gtfs
                        .shapes
                        .into_iter()
                        .map(|(shape_id, shapes)| {
                            let coords = shapes
                                .into_iter()
                                .map(|shape| Coord {
                                    x: shape.longitude,
                                    y: shape.latitude,
                                })
                                .collect_vec();
                            (record, shape_id, LineString::new(coords))
                        })
                        .collect_vec();

                    println!("{} - {} shape rows", record.provider, rows.len());
                    rows
                }
            },
        })
        .collect::<Vec<_>>();

    let mut out = File::create_new("gtfs_shapes.csv").unwrap();
    writeln!(out, "provider,url,state_code,shape_id,geometry").unwrap();

    for (record, shape_id, linestring) in results {
        writeln!(out, "{},{},\"{}\"", record, shape_id, linestring.to_wkt()).unwrap();
    }
}

fn download(rows: &[GtfsProvider], parallelism: usize) {
    let par_16: u16 = parallelism.try_into().unwrap();
    let downloads = rows
        .iter()
        .sorted_by_cached_key(|row| row.filename())
        .dedup_by(|a, b| a.filename() == b.filename())
        .flat_map(|row| {
            row.url.clone().map(|url| {
                let filename = row.filename();
                let filepath = Path::new(&filename);
                downloader::Download::new(&url).file_name(filepath)
            })
        })
        .collect_vec();

    let mut downloader = downloader::downloader::Builder::default()
        .connect_timeout(Duration::from_secs(10))
        .download_folder(Path::new("."))
        .parallel_requests(par_16)
        .build()
        .unwrap();

    let result = downloader.download(&downloads).unwrap();
    for row in result {
        match row {
            Ok(_) => {}
            Err(e) => log::error!("{e}"),
        }
    }
}
