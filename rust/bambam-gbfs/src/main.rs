use bambam_gbfs::app::GbfsCliArguments;
use clap::Parser;

fn main() {
    env_logger::init();
    let args = GbfsCliArguments::parse();
    match args.op.run() {
        Ok(_) => log::info!("finished."),
        Err(e) => {
            log::error!("failed running bambam_gbfs: {e}");
            std::process::exit(1);
        }
    }
}
