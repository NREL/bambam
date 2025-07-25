use super::MepRow;
use bamsoda_core::model::identifier::{Geoid, HasGeoidString};
use geo::Geometry;
use serde::{Deserialize, Serialize};
use wkt::ToWkt;

/// a row of MEP data aggregated to some Geoid geometry
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutRow {
    pub geoid: String,
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
    pub fn new(geoid: Geoid, geometry: &Geometry, rows: &[MepRow]) -> Self {
        let mut out_row = OutRow::empty(geoid.clone(), geometry);
        for row in rows.iter() {
            out_row.add(row);
        }
        out_row
    }

    /// sets up the OutRow with empty accumulators
    pub fn empty(geoid: Geoid, geometry: &Geometry) -> Self {
        OutRow {
            geoid: geoid.geoid_string(),
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
        self.mep += mep_row.mep;
        self.mep_entertainment += mep_row.mep_entertainment;
        self.mep_food += mep_row.mep_food;
        self.mep_healthcare += mep_row.mep_healthcare;
        self.mep_jobs += mep_row.mep_jobs;
        self.mep_retail += mep_row.mep_retail;
        self.mep_services += mep_row.mep_services;
        self.population += mep_row.population;
    }
}
