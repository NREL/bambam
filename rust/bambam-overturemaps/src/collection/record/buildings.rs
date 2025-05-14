use geo::Geometry;
use serde::{Deserialize, Serialize};

use super::deserialize_geometry;
use super::{OvertureMapsBbox, OvertureMapsNames, OvertureMapsSource, RecordDataset};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
// #[serde(deny_unknown_fields)]
pub struct BuildingsRecord {
    id: Option<String>,
    #[serde(deserialize_with = "deserialize_geometry")]
    geometry: Option<Geometry>,
    bbox: OvertureMapsBbox,
    version: i32,
    sources: Option<Vec<Option<OvertureMapsSource>>>,
    names: Option<OvertureMapsNames>,
    subtype: Option<String>,
    class: Option<String>,
    level: Option<i32>,
    has_parts: Option<bool>,
    is_underground: Option<bool>,
    height: Option<f64>,
    num_floors: Option<i32>,
    num_floors_underground: Option<i32>,
    min_height: Option<f64>,
    min_floor: Option<i32>,
    facade_color: Option<String>,
    facade_material: Option<String>,
    roof_material: Option<String>,
    roof_shape: Option<String>,
    roof_direction: Option<f64>,
    roof_orientation: Option<String>,
    roof_color: Option<String>,
}

impl RecordDataset for BuildingsRecord {
    type Record = BuildingsRecord;

    fn format_url(release_str: String) -> String {
        format!("release/{release_str}/theme=buildings/type=building/").to_owned()
    }
}

impl BuildingsRecord {
    pub fn get_class(&self) -> Option<String> {
        self.class.clone()
    }

    pub fn get_geometry(&self) -> Option<Geometry> {
        self.geometry.clone()
    }
}
