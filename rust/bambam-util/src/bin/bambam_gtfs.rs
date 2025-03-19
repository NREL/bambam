/*
    this script reads all GTFS archives in the MobilityData catalog and collects information
    about the trips, WKT shapes, and trip legs of the collection.
*/

use bambam_util::gtfs::{app::GtfsApp, GtfsProvider};
use clap::Parser;
use kdam::tqdm;
use std::path::PathBuf;

fn main() {
    let args = GtfsApp::parse();
    let path_buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("resources")
        .join(&args.manifest_file);
    let reader = csv::ReaderBuilder::new()
        .from_path(path_buf.as_path())
        .unwrap_or_else(|_| panic!("file {} not found", path_buf.to_str().unwrap_or_default()));
    let row_iter = tqdm!(
        reader.into_deserialize::<GtfsProvider>(),
        desc = format!("reading {}", args.manifest_file)
    );
    let us_rows: Vec<GtfsProvider> = row_iter
        .filter(|r| match r {
            Ok(provider) => provider.country_code == *"US" && provider.data_type == "gtfs",
            Err(_) => true,
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    args.op.run(&us_rows, args.parallelism)
}
