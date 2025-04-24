use geo::Geometry;
use geozero::{wkb::Wkb, ToGeo};
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde::de::Deserializer;
use serde::de::DeserializeOwned;


pub trait RecordDataset{
    type Record: DeserializeOwned + Send;
    fn format_url(release_str: String) -> String;
}


pub fn deserialize_geometry<'de, D>(deserializer: D) -> Result<Option<Geometry>, D::Error>
where
    D: Deserializer<'de>,
{
    let hex_str: String = Option::deserialize(deserializer)?.unwrap();
    let wkb_bytes: Vec<u8> = hex::decode(&hex_str).ok().unwrap();
    Ok(Wkb(&wkb_bytes).to_geo().ok())
}


#[derive(Debug, Serialize, Deserialize)]
pub struct OvertureMapsBbox{
    xmin: Option<f32>,
    xmax: Option<f32>,
    ymin: Option<f32>,
    ymax: Option<f32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OvertureMapsSource{
    property: Option<String>,
    dataset: Option<String>,
    record_id: Option<String>,
    update_time: Option<String>,
    confidence: Option<f64>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OvertureMapsNames{
    primary: Option<String>,
    common: Option<HashMap<String, Option<String>>>,
    rules: Option<Vec<OvertureMapsNamesRule>>
}

#[derive(Debug, Serialize, Deserialize)]
struct OvertureMapsNamesRule{
    variant: Option<String>, 
    language: Option<String>,
    value: Option<String>,
    between: Option<Vec<f64>>,
    side: Option<String>
}