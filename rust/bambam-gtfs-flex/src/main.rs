use std::path::Path;

mod calendar;
mod flex_processor;
mod locations;
mod stop_times;
mod trips;

use crate::flex_processor::process_gtfs_flex_bundle;

fn main() -> std::io::Result<()> {
    // feeds path directory
    let flex_dir = Path::new("../../gtfs_flex_feeds");

    // requested date and time for processing GTFS-Flex feeds
    let date_requested = "20240902";
    let time_requested = "09:00:00";

    process_gtfs_flex_bundle(flex_dir, date_requested, time_requested)?;

    Ok(())
}
