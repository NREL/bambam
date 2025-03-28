use clap::command;
use routee_compass::app::cli::cli_args::CliArgs;
use serde::{Deserialize, Serialize};

use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct BambamAccessApp {
    #[command(subcommand)]
    app: App,
}

#[derive(Subcommand)]
pub enum App {
    /// run all phases (search, reachability, scoring) of the bambam app
    #[command()]
    All {
        /// RouteE Compass service configuration TOML file
        // #[arg(short, long, value_name = "*.toml")]
        config_file: String,

        /// JSON file containing queries. Should be newline-delimited if chunksize is set
        // #[arg(short, long, value_name = "*.json")]
        query_file: String,

        /// Size of batches to load into memory at a time
        // #[arg(long)]
        chunksize: Option<i64>,

        /// Format of JSON queries file, if regular JSON or newline-delimited JSON
        // #[arg(short, long)]
        newline_delimited: bool,
    },
    // Search,
    // Reachability,
    /// run only the scoring phase, assuming some output dataset of reachability data
    #[command()]
    Scoring,
}

/// duplicates the RouteE Compass app

pub struct ScoringApp {}
