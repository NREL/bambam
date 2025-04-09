use csv;
use itertools::Itertools;
use kdam::tqdm;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InputRow {
    latitude: f64,
    longitude: f64,
    propertytype: Option<String>,
    propertysubtype: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OutputRow {
    latitude: f64,
    longitude: f64,
    activity_type: String,
}

impl OutputRow {
    pub fn new(lat: f64, lon: f64, cat: String) -> OutputRow {
        OutputRow {
            latitude: lat,
            longitude: lon,
            activity_type: cat,
        }
    }
}

/// single-use script for converting our legacy CoStar data into a long-format
/// CSV with latitude, longitude, and activity_type columns, using the MEP 1
/// CoStar activity mapping.
fn main() {
    let mut reader = csv::Reader::from_path(Path::new(
        "/Users/rfitzger/data/mep/mep3/input/opportunities/2018-04-costar.csv",
    ))
    .unwrap();

    let process_iter = tqdm!(reader.deserialize(), desc = "reading and filtering rows");
    let result = process_iter
        .flat_map(|record| {
            let row: InputRow = record.unwrap();
            let category = mapping(row.propertytype.as_deref(), row.propertysubtype.as_deref());
            category.map(|cat| OutputRow::new(row.latitude, row.longitude, cat))
        })
        .collect_vec();
    eprintln!();

    let mut writer = csv::Writer::from_path(
        "/Users/rfitzger/data/mep/mep3/input/opportunities/2018-04-costar-mep-long.csv",
    )
    .unwrap();

    let n_results = result.len();
    let write_iter = tqdm!(
        result.into_iter(),
        desc = "writing result",
        total = n_results
    );
    for row in write_iter {
        writer.serialize(&row).unwrap();
    }
    eprintln!();
    println!("finished.");
}

/// mapping from MEP 1
///  landUse$propertytype[landUse$propertytype=='Health Care'] <- Default.OpportunityTypes$health
///  landUse$propertytype[landUse$propertytype=='Sports & Entertainment'] <- Default.OpportunityTypes$entertainment
///  landUse$propertytype[landUse$propertytype=='Retail'] <- Default.OpportunityTypes$retail
///  landUse$propertytype[landUse$propertytype=='Specialty'] <- Default.OpportunityTypes$services
///  landUse$propertytype[landUse$propertysubtype %in% c('Fast Food', 'Restaurant', 'Bar')] <- Default.OpportunityTypes$food
fn mapping(ptype: Option<&str>, psubtype: Option<&str>) -> Option<String> {
    match (ptype, psubtype) {
        (Some("Health Care"), _) => Some(String::from("healthcare")),
        (Some("Sports & Entertainment"), _) => Some(String::from("entertainment")),
        (Some("Retail"), _) => Some(String::from("retail")),
        (Some("Specialty"), _) => Some(String::from("services")),
        (_, Some("Fast Food")) => Some(String::from("food")),
        (_, Some("Restaurant")) => Some(String::from("food")),
        (_, Some("Bar")) => Some(String::from("food")),
        _ => None,
    }
}
