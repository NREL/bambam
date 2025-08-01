use crate::app::overlay::Grouping;

use super::MepRow;
use bamsoda_core::model::identifier::{Geoid, HasGeoidString};
use geo::Geometry;
use serde::{Deserialize, Serialize};
use wkt::ToWkt;

/// a row of MEP data aggregated to some Geoid geometry
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutRow {
    pub geoid: String,
    pub mode: String,
    pub geometry: String,
    pub mep: f64,
    pub mep_entertainment: f64,
    pub mep_food: f64,
    pub mep_healthcare: f64,
    pub mep_jobs: f64,
    pub mep_retail: f64,
    pub mep_services: f64,
    pub population: f64, // currently missing from rows
}

impl OutRow {
    pub fn new(grouping: &Grouping, geometry: &Geometry, rows: &[MepRow]) -> Self {
        let mut out_row = OutRow::empty(grouping.clone(), geometry);
        for row in rows.iter() {
            out_row.add(row);
        }
        out_row
    }

    /// sets up the OutRow with empty accumulators
    pub fn empty(grouping: Grouping, geometry: &Geometry) -> Self {
        OutRow {
            geoid: grouping.geoid.geoid_string(),
            mode: grouping.mode.clone(),
            geometry: geometry.to_wkt().to_string(),
            mep: Default::default(),
            mep_entertainment: Default::default(),
            mep_food: Default::default(),
            mep_healthcare: Default::default(),
            mep_jobs: Default::default(),
            mep_retail: Default::default(),
            mep_services: Default::default(),
            population: Default::default(),
        }
    }

    pub fn add(&mut self, mep_row: &MepRow) {
        self.mep += mep_row.mep.unwrap_or_default();
        self.mep_entertainment += mep_row.mep_entertainment.unwrap_or_default();
        self.mep_food += mep_row.mep_food.unwrap_or_default();
        self.mep_healthcare += mep_row.mep_healthcare.unwrap_or_default();
        self.mep_jobs += mep_row.mep_jobs.unwrap_or_default();
        self.mep_retail += mep_row.mep_retail.unwrap_or_default();
        self.mep_services += mep_row.mep_services.unwrap_or_default();
        self.population += mep_row.population.unwrap_or_default();
    }
}
