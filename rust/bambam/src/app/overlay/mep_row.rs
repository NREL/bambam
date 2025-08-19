use serde::{Deserialize, Serialize};

/// a row of raw MEP data.
///
/// CSV rows as currently defined:
/// grid_id,isochrone_10,isochrone_20,isochrone_30,isochrone_40,lat,lon,
/// mep,mep_entertainment,mep_food,mep_healthcare,mep_jobs,mep_retail,mep_services,
/// mode,opps_entertainment_10,opps_entertainment_20,opps_entertainment_30,opps_entertainment_40,opps_entertainment_total,
/// opps_food_10,opps_food_20,opps_food_30,opps_food_40,opps_food_total,
/// opps_healthcare_10,opps_healthcare_20,opps_healthcare_30,opps_healthcare_40,opps_healthcare_total,
/// opps_jobs_10,opps_jobs_20,opps_jobs_30,opps_jobs_40,opps_jobs_total,
/// opps_retail_10,opps_retail_20,opps_retail_30,opps_retail_40,opps_retail_total,opps_services_10,
/// opps_services_20,opps_services_30,opps_services_40,opps_services_total,
/// population,ram_mb,runtime_iter_opps,runtime_mep,runtime_opps,runtime_search
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MepRow {
    pub grid_id: String,
    pub lat: f64,
    pub lon: f64,
    pub mode: String,
    pub mep: Option<f64>,
    pub mep_entertainment: Option<f64>,
    pub mep_food: Option<f64>,
    pub mep_healthcare: Option<f64>,
    pub mep_jobs: Option<f64>,
    pub mep_retail: Option<f64>,
    pub mep_services: Option<f64>,
    pub population: Option<f64>, // currently missing from rows
}
