use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use bambam::model::builder;
use clap::Parser;
use routee_compass::app::cli::cli_args::CliArgs;
use routee_compass::app::compass::{CompassApp, CompassAppError, CompassJsonExtensions};
use serde_json::Value;
inventory::submit! { builder::BUILDER_REGISTRATION }

// Import the library to ensure inventory registrations in lib.rs are included
#[allow(unused_imports)]

fn main() {
    env_logger::init();

    log::debug!("cwd: {:?}", std::env::current_dir());
    let args = CliArgs::parse();
    // let args = CliArgs {
    //     config_file: String::from("test.toml"),
    //     query_file: String::from("test.json"),
    //     chunksize: None,
    //     newline_delimited: false,
    // };
    match run_bambam(args) {
        Ok(_) => {}
        Err(e) => log::error!("{e}"),
    }
}

fn run_bambam(args: CliArgs) -> Result<Vec<Value>, CompassAppError> {
    log::info!("starting app at {}", chrono::Local::now().to_rfc3339());
    let config_path = Path::new(&args.config_file);
    let app = CompassApp::try_from(config_path)?;
    let query_filename = &args.query_file;
    let query_file = File::open(query_filename).map_err(|e| {
        CompassAppError::BuildFailure(format!(
            "failure reading input query file '{query_filename}': {e}"
        ))
    })?;
    let reader = BufReader::new(query_file);
    let user_json: serde_json::Value = serde_json::from_reader(reader)?;
    let mut user_queries = user_json.get_queries()?;
    app.run(&mut user_queries, None)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use routee_compass::app::cli::cli_args::CliArgs;

    use crate::run_bambam;

    #[test]
    fn test_e2e_denver() {
        let conf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("test invariant failed: repo 'rust/bambam' dir has no parent")
            .parent()
            .expect("test invariant failed: repo 'rust' dir has no parent")
            .join("configuration")
            .join("denver_test.toml");
        let query_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("test invariant failed: repo 'rust/bambam' dir has no parent")
            .parent()
            .expect("test invariant failed: repo 'rust' dir has no parent")
            .join("query")
            .join("denver_extent.json");
        let conf_file = conf_path
            .to_str()
            .expect("test invariant failed: config file path cannot be a string");
        let query_file = query_path
            .to_str()
            .expect("test invariant failed: config file path cannot be a string");

        let args = CliArgs {
            config_file: conf_file.to_string(),
            query_file: query_file.to_string(),
            chunksize: None,
            newline_delimited: false,
        };
        let expected_grid_size = 48;

        match run_bambam(args) {
            Ok(rows) => {
                assert_eq!(rows.len(), expected_grid_size);
                for (idx, row) in rows.iter().enumerate() {
                    if let Some(error) = row.get("error") {
                        panic!(
                            "row {idx} has error: {}",
                            serde_json::to_string_pretty(error).unwrap_or_default()
                        );
                    }
                }
            }
            Err(e) => panic!("test failed: {e}"),
        }
    }
}
