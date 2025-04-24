use super::{source::lodes::lodes_ops, study_region::StudyRegion};
use bamsoda_app::app::lodes_tiger;
use bamsoda_core::model::identifier::GeoidType;
use bamsoda_lehd::model::{LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment};
use geo::Geometry;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bambam_overturemaps::collection::{ api::CollectionRunConfig, OvertureMapsCollectorConfig, TaxonomyModelBuilder };


/// an API data source for opportunities.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpportunitySource {
    /// collects opportunities from a Longitudinal Employer-Household Dynamics (LODES)
    /// Workplace Area Characteristics (WAC) dataset paired with it's corresponding
    /// TIGER/Line Shapefile. the user provides a mapping from each WacSegment to a list of
    /// activity types (at least one) which it represents.
    #[serde(rename = "lodes")]
    UsCensusLehdLodes {
        activity_mapping: HashMap<WacSegment, Vec<String>>,
        study_region: StudyRegion,
        data_granularity: Option<GeoidType>,
        edition: LodesEdition,
        job_type: LodesJobType,
        segment: WorkplaceSegment,
        year: u64,
    },
    /// collects opportunities from <https://docs.overturemaps.org/guides/places/>.
    #[serde(rename = "overture")]
    OvertureMapsPlaces {
        taxonomy_mapping: TaxonomyModelBuilder,
        collector_config: OvertureMapsCollectorConfig,
        run_config: CollectionRunConfig
    },
}

impl OpportunitySource {
    /// generates a collection of Geometries paired with activity counts
    /// from some data source API. Configurations for a given API are
    /// provided by this [`OpportunitySource`] instance.
    ///
    /// # Arguments
    ///
    /// * `activity_types` - the types of activities expected
    ///
    /// # Returns
    ///
    /// A collection of Geometries tagged with activity rows.
    pub fn generate_dataset(
        &self,
        activity_types: &[String],
    ) -> Result<Vec<(Geometry, Vec<f64>)>, String> {
        match self {
            OpportunitySource::OvertureMapsPlaces {
                ..
            } => todo!(),
            OpportunitySource::UsCensusLehdLodes {
                activity_mapping,
                study_region,
                data_granularity,
                edition,
                job_type,
                segment,
                year,
            } => {
                //
                let geoids = study_region.get_geoids()?;
                let dataset = LodesDataset::WAC {
                    edition: *edition,
                    job_type: *job_type,
                    segment: *segment,
                    year: *year,
                };
                let wac_segments = activity_mapping.keys().cloned().collect_vec();
                lodes_ops::collect_lodes_opportunities(
                    &dataset,
                    &wac_segments,
                    &geoids,
                    data_granularity,
                    activity_types,
                    activity_mapping,
                )
            }
        }
    }
}
