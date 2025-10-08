use super::GtfsOperation;
use clap::Parser;

/// command line tool for batch downloading and summarizing of GTFS archives
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct GtfsApp {
    #[command(subcommand)]
    pub op: GtfsOperation,
}
