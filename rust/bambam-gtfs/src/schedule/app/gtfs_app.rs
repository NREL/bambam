use super::GtfsOperation;
use clap::Parser;

/// command line tool for batch downloading and summarizing of GTFS archives
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct GtfsApp {
    #[command(subcommand)]
    pub op: GtfsOperation,
    #[arg(long, default_value_t = 1)]
    pub parallelism: usize,
    #[arg(long, default_value_t=String::from("2024-08-13-mobilitydataacatalog.csv"))]
    pub manifest_file: String,
}
