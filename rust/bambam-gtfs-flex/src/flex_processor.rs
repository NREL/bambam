use crate::calendar::{read_calendar_from_flex, Calendar};
use crate::locations::{read_locations_from_flex, Location};
use crate::stop_times::{read_stop_times_from_flex, StopTimes};
use crate::trips::{read_trips_from_flex, Trips};

use chrono::Datelike;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

// pub struct CsvTable {
//     pub header: StringRecord,
//     pub rows: Vec<StringRecord>,
// }

// impl CsvTable {
//     pub fn col_idx(&self, name: &str) -> usize {
//         self.header
//             .iter()
//             .position(|h| h == name)
//             .unwrap_or_else(|| panic!("Column '{}' not found", name))
//     }
// }

pub fn process_gtfs_flex_bundle(flex_directory_path: &Path) -> io::Result<()> {
    println!("=== Processing GTFS-Flex bundle ===");

    // discover gtfs-flex feeds
    discover_gtfs_flex_feeds(flex_directory_path)?;

    // process files in each feed
    process_flex_files(flex_directory_path)?;

    println!("=== GTFS-Flex processing complete ===");
    Ok(())
}

/// discover all zip files in the given directory
pub fn discover_gtfs_flex_feeds(flex_directory_path: &Path) -> io::Result<()> {
    if !flex_directory_path.exists() {
        eprintln!("Directory does not exist: {:?}", flex_directory_path);
        return Ok(());
    }

    let entries = fs::read_dir(flex_directory_path)?;

    println!("Found zip files in {:?}:", flex_directory_path);

    let mut count = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "zip") {
            if let Some(name) = path.file_name() {
                println!("      {}", name.to_string_lossy());
                count += 1;
            }
        }
    }

    println!("Total GTFS-flex feeds found: {}", count);

    Ok(())
}

/// iterate over gtfs-flex feeds and process files from each feed
pub fn process_flex_files(flex_directory_path: &Path) -> io::Result<()> {
    println!("Processing GTFS-Flex feeds in {:?}", flex_directory_path);

    for entry in std::fs::read_dir(flex_directory_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "zip") {
            println!("  Processing {:?}", path);

            // read calender.txt
            let calendar = read_calendar_from_flex(&path)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Error in calendar.txt")
            })?;
            // println!("      calendar.txt records: {:?}", calendar);
            println!("      calendar.txt read!");

            // read trips.txt
            let trips = read_trips_from_flex(&path)?
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Error in trips.txt"))?;
            // println!("      trips.txt records: {:?}", trips);
            println!("      trips.txt read!");

            // read stop_times.txt
            let stop_times = read_stop_times_from_flex(&path)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Error in stop_times.txt")
            })?;
            // println!("      stop_times.txt records: {:?}", stop_times);
            println!("      stop_times.txt read!");

            // read locations.geojson
            let locations = read_locations_from_flex(&path)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Error in locations.geojson")
            })?;
            // println!("      locations.geojson content: {:?}", locations);
            println!("      locations.geojson read!");

            // process files
            let date_requested = "20240902"; // example date
            let time_requested = "09:00:00"; // example time
            join_flex_files(
                &calendar,
                &trips,
                &stop_times,
                &locations,
                date_requested,
                time_requested,
            )?;
        }
    }

    println!("GTFS-Flex feeds processed!");

    Ok(())
}

/// process calender, trips, stop_times, and locaitons files for the requested date and time
pub fn join_flex_files(
    calendar: &[Calendar],
    trips: &[Trips],
    stop_times: &[StopTimes],
    _locations: &[Location],
    date_requested: &str,
    time_requested: &str,
) -> io::Result<()> {
    use chrono::NaiveTime;

    // parse requested date and time
    let date = chrono::NaiveDate::parse_from_str(date_requested, "%Y%m%d")
        .expect("Invalid date format YYYYMMDD");
    let time = NaiveTime::parse_from_str(time_requested, "%H:%M:%S")
        .expect("Invalid time format HH:MM:SS");
    println!("          requested date: {:?}", date);
    println!("          requested time: {:?}", time);

    // filter calendar for the requested date
    let weekday = match date.weekday() {
        chrono::Weekday::Mon => |c: &Calendar| c.monday == 1,
        chrono::Weekday::Tue => |c: &Calendar| c.tuesday == 1,
        chrono::Weekday::Wed => |c: &Calendar| c.wednesday == 1,
        chrono::Weekday::Thu => |c: &Calendar| c.thursday == 1,
        chrono::Weekday::Fri => |c: &Calendar| c.friday == 1,
        chrono::Weekday::Sat => |c: &Calendar| c.saturday == 1,
        chrono::Weekday::Sun => |c: &Calendar| c.sunday == 1,
    };
    println!("          requested day: {:?}", date.weekday());

    let active_service_ids: Vec<&str> = calendar
        .iter()
        .filter(|c| weekday(c) && c.start_date <= date && date <= c.end_date)
        .map(|c| c.service_id.as_str())
        .collect();

    println!("          active service_ids: {:?}", active_service_ids);

    // filter trips by active service_ids
    let active_trips: Vec<&Trips> = trips
        .iter()
        .filter(|t| active_service_ids.contains(&t.service_id.as_str()))
        .collect();

    println!("          active trips: {:?}", active_trips);

    // filter stop_times for active trips and by requested time
    let active_trip_ids: Vec<&str> = active_trips.iter().map(|t| t.trip_id.as_str()).collect();

    let active_stop_times: Vec<&StopTimes> = stop_times
        .iter()
        .filter(|st| {
            active_trip_ids.contains(&st.trip_id.as_str())
                && st.start_pickup_drop_off_window <= time
                && time <= st.end_pickup_drop_off_window
        })
        .collect();

    println!("          active stop_times: {:?}", active_stop_times);

    // create valid zones of origin-destination pairs from each trip in stop_times
    let mut valid_zones: HashMap<String, (String, String)> = HashMap::new();

    for trip_id in active_stop_times.iter().map(|st| &st.trip_id) {
        // filter stop_times for this trip_id
        let trip_stop_times: Vec<&&StopTimes> = active_stop_times
            .iter()
            .filter(|st| &st.trip_id == trip_id)
            .collect();

        // find origin: pickup allowed, dropoff not allowed
        let origin = trip_stop_times
            .iter()
            .find(|st| st.pickup_type == 2 && st.drop_off_type == 1)
            .map(|st| st.location_id.clone())
            .unwrap_or_else(|| "".to_string());

        // find destination: pickup not allowed, dropoff allowed
        let destination = trip_stop_times
            .iter()
            .find(|st| st.pickup_type == 1 && st.drop_off_type == 2)
            .map(|st| st.location_id.clone())
            .unwrap_or_else(|| "".to_string());

        if !origin.is_empty() && !destination.is_empty() {
            valid_zones.insert(trip_id.clone(), (origin, destination));
        }
    }
    println!(
        "Valid zones (trip_id -> (origin, destination)): {:?}",
        valid_zones
    );

    // struct type for valid zones
    // then
    // add location geometries to valid zones
    // may be add geometries in the single step above

    println!("    GTFS-Flex files processed successfully!");

    Ok(())
}
