use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Overlay {
    Intersection,
}

impl ToString for Overlay {
    fn to_string(&self) -> String {
        match self {
            Self::Intersection => String::from("intersection"),
        }
    }
}
