use geo::Geometry;
use std::sync::Arc;
use rayon::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use bambam_overturemaps::collection::{ Bbox, OvertureMapsCollectionError, OvertureMapsCollector, PlacesRecord };
use bambam_overturemaps::collection::{ OvertureMapsCollectorConfig, TaxonomyModelBuilder, TaxonomyModel, RowFilterConfig, ReleaseVersion};

#[derive(Debug)]
pub struct OvertureOpportunityCollectionModel{
    collector: OvertureMapsCollector,
    release_version: ReleaseVersion,
    row_filter_config: Option<RowFilterConfig>,
    taxonomy_model: Arc<TaxonomyModel>
}

impl OvertureOpportunityCollectionModel {

    pub fn new(
        collector_config: OvertureMapsCollectorConfig,
        release_version: ReleaseVersion,
        bbox_boundary: Bbox,
        activity_mappings: HashMap<String, Vec<String>>
    ) -> Result<Self, OvertureMapsCollectionError>{
        let taxonomy_model = Arc::new(TaxonomyModelBuilder::new(activity_mappings.clone(), None).build()?);
        let row_filter_config = RowFilterConfig::Combined {
            filters: vec![
                Box::new(bbox_boundary.into()),
                Box::new(RowFilterConfig::from(activity_mappings))
            ]
        };
        
        Ok(
            Self {
                collector: OvertureMapsCollector::try_from(collector_config)?,
                release_version: release_version,
                row_filter_config: Some(row_filter_config),
                taxonomy_model: taxonomy_model
            }
        )
    }

    pub fn collect(&self, activity_types: &[String]) -> Result<Vec<(Geometry, Vec<f64>)>, OvertureMapsCollectionError>{
        let records = self.collector.collect_from_release::<PlacesRecord>(
            self.release_version.clone(),
            self.row_filter_config.clone()
        )?;

        // Compute MEP category vectors
        let mep_vectors = records.par_iter().map(|record| {
            Ok::<Vec<f64>, OvertureMapsCollectionError>(
                self.taxonomy_model.clone()
                    .reverse_map(record.get_categories(), activity_types.to_vec())?
                    // Reduce Vec<Vec<bool>> to Vec<bool> applying OR logic
                    .into_iter()
                    .reduce(|mut acc, v|{
                        acc.iter_mut()
                            .zip(v.iter())
                            .for_each(|(a,b )| *a |= b);
                        acc    
                    })
                    .unwrap_or_default()
                    // Map bool to f64
                    .into_par_iter()
                    .map(|v| v as i16 as f64)
                    .collect()
                )
        }).collect::<Result<Vec<Vec<f64>>, OvertureMapsCollectionError>>()?;


        // Collect POI geometries
        let mep_geometries: Vec<Option<Geometry>> = records.into_par_iter()
                .map(|record| record.get_geometry())
                .collect();

        // Zip geometries and vectors
        Ok(mep_geometries.into_iter()
            .zip(mep_vectors)
            .filter_map(|(maybe_geometry, vector)| {
              maybe_geometry.map(|geometry| (geometry, vector))
            }).collect::<Vec<(Geometry, Vec<f64>)>>())
    }
}

