use std::path::Path;

mod calendar;
mod flex_processor;
mod locations;
mod stop_times;
mod trips;

use crate::flex_processor::process_gtfs_flex_bundle;

fn main() -> std::io::Result<()> {
    let flex_dir = Path::new("../../gtfs_flex_feeds");

    process_gtfs_flex_bundle(flex_dir)?;

    Ok(())
}
