use clap::Parser;
use routee_compass::app::cli::{cli_args::CliArgs, run};
use routee_compass::app::compass::{CompassAppError};

// Import the library to ensure inventory registrations in lib.rs are included
#[allow(unused_imports)]
use bambam;

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

fn run_bambam(args: CliArgs) -> Result<(), CompassAppError> {
    log::info!("starting app at {}", chrono::Local::now().to_rfc3339());

    match run::command_line_runner(&args, None, None) {
        Ok(_) => {}
        Err(e) => {
            log::error!("{e}")
        }
    }

    Ok(())
}
