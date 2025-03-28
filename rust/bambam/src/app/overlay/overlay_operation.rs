use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum OverlayOperation {
    Intersection,
}

impl ToString for OverlayOperation {
    fn to_string(&self) -> String {
        match self {
            Self::Intersection => String::from("intersection"),
        }
    }
}
