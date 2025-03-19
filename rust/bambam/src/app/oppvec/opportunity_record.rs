use geo::Point;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum OpportunityRecord {
    Wide,
    Long {
        geometry: Point<f64>,
        category: String,
    },
}
