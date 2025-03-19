use crate::gtfs::{GtfsProvider, GtfsSummary};
use clap::{Subcommand, ValueEnum};
use geo::{Coord, LineString};
use gtfs_structures::Gtfs;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::File, io::Write, path::Path, time::Duration};
use wkt::ToWkt;

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum, Subcommand)]
pub enum GtfsOperation {
    /// summarize attributes for the downloaded GTFS archives
    Summary,
    /// download all WKT shapes data from the GTFS archives
    Shapes,
    /// download all of the GTFS archives
    Download,
}

impl GtfsOperation {
    pub fn run(&self, rows: &Vec<GtfsProvider>, parallelism: usize) {
        match self {
            GtfsOperation::Summary => summarize(rows),
            GtfsOperation::Shapes => shapes(rows),
            GtfsOperation::Download => download(rows, parallelism),
        }
    }
}

fn summarize(rows: &Vec<GtfsProvider>) {
    let results = rows
        .par_iter()
        .map(|record| match &record.url {
            None => Ok((record, GtfsSummary::default())),
            Some(url) => match Gtfs::new(url) {
                Err(e) => Ok((record, GtfsSummary::error(format!("gtfs error: {}", e)))),
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
                            let has_shape = &gtfs.shapes.get(&shape_id).is_some();
                            if *has_shape {
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
        writeln!(out, "{},{}", record, summary).unwrap();
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

fn download(rows: &Vec<GtfsProvider>, parallelism: usize) {
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
            Err(e) => log::error!("{}", e),
        }
    }
}
