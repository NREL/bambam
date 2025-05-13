use compass_tomtom::model::traversal::time_of_day::record;
use geo::{centroid, Centroid, Geometry};
use routee_compass_core::util::geo::PolygonalRTree;
use std::sync::Arc;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use bambam_overturemaps::collection::{ Bbox, BuildingsRecord, OvertureMapsCollectionError, OvertureMapsCollector, PlacesRecord, RecordDataset };
use bambam_overturemaps::collection::{ OvertureMapsCollectorConfig, TaxonomyModelBuilder, TaxonomyModel, RowFilterConfig, ReleaseVersion};

#[derive(Debug)]
pub struct OvertureOpportunityCollectionModel{
    collector: OvertureMapsCollector,
    release_version: ReleaseVersion,
    places_row_filter_config: Option<RowFilterConfig>,
    buildings_row_filter_config: Option<RowFilterConfig>,
    places_taxonomy_model: Arc<TaxonomyModel>,
    buildings_activity_mappings: HashMap<String, Vec<String>>
}

impl OvertureOpportunityCollectionModel {

    pub fn new(
        collector_config: OvertureMapsCollectorConfig,
        release_version: ReleaseVersion,
        bbox_boundary: Bbox,
        places_activity_mappings: HashMap<String, Vec<String>>,
        buildings_activity_mappings: HashMap<String, Vec<String>>
    ) -> Result<Self, OvertureMapsCollectionError>{
        let taxonomy_model = Arc::new(TaxonomyModelBuilder::new(places_activity_mappings.clone(), None).build()?);
        let places_row_filter_config = RowFilterConfig::Combined {
            filters: vec![
                Box::new(RowFilterConfig::from(bbox_boundary)),
                Box::new(RowFilterConfig::from(places_activity_mappings))
            ]
        };
        let buildings_row_filter_config = RowFilterConfig::Combined {
            filters: vec![
                Box::new(RowFilterConfig::from(bbox_boundary)),
                Box::new(RowFilterConfig::HasClass)
            ]
        };
        
        Ok(
            Self {
                collector: OvertureMapsCollector::try_from(collector_config)?,
                release_version: release_version,
                places_row_filter_config: Some(places_row_filter_config),
                buildings_row_filter_config: Some(buildings_row_filter_config),
                places_taxonomy_model: taxonomy_model,
                buildings_activity_mappings: buildings_activity_mappings
            }
        )
    }

    /// Collect opportunities from Places and Buildings datasets and
    /// process them into Vec<[`Geometry`], f64> according to the configuration of the model
    pub fn collect(&self, activity_types: &[String]) -> Result<Vec<(Geometry, Vec<f64>)>, OvertureMapsCollectionError>{
        // Collect raw opportunities
        let mut places_opportunities = self.collect_places_opportunities(activity_types)?;
        let buildings_opportunities = self.collect_building_opportunities(activity_types)?;

        // Build RTree for places
        let rtree = PolygonalRTree::new(
            places_opportunities.iter()
                .enumerate()
                .map(|(i, (geom, _))| (geom.clone(), i))
                .collect::<Vec<(Geometry, usize)>>()
        )
        .map_err(|e| OvertureMapsCollectionError::ProcessingError(e))?;

        // For each building, we are going to:
        //  1. Compute the intersection with places points
        //  2. Compare the MEP vectors
        //  3. If the building has a category not contained in the places data
        //     we return it as a new opportunity. Otherwise we skip it.
        let mut filtered_buildings: Vec<(Geometry, Vec<bool>)> = buildings_opportunities
            .into_par_iter()
            .map(|building|{
                // Aggregate the values of all matching points into a single MEP vector
                let places_mep_agg = rtree
                    .intersection(&building.0)?
                    // For each returned index in the intersection, find the corresponding opportunity tuple (Geometry, Vec<bool>)
                    .filter_map(|node| places_opportunities.get(node.data))
                    // Reduce them to a single Vec<bool> using an OR operation
                    .fold(
                        vec![false; activity_types.len()],
                        |mut acc, row| {
                            for (a, &b) in acc.iter_mut().zip(&row.1) {
                                *a |= b;
                            }
                            acc
                        });
                
                // TODO: This logic potentially duplicates an opportunity, but was the logic implemented by the researchers
                // Compare node (Places) MEP vector to building MEP vector
                // We want to know if for any MEP category of the building is not contained in the points
                let keep_building = building.1
                    .iter().zip(places_mep_agg)
                    .any(|(b_flag, p_flag)| b_flag & !p_flag);
                
                // Compute centroid if available
                let centroid = building.0.centroid();

                Ok::<_, String>(
                    if keep_building { centroid.map(|p| (p.into(), building.1)) }
                    else { None }
                )
            })
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| OvertureMapsCollectionError::ProcessingError(e))?;

        // Merge places_opportunities + buildings.centroid
        places_opportunities.extend(filtered_buildings.into_iter());

        Ok(
            places_opportunities
                .into_par_iter()
                .map(|(g, vec)| (g, vec.into_iter().map(|v| v as i16 as f64).collect()))
                .collect()
        )
    }

    fn collect_places_opportunities(&self, activity_types: &[String]) -> Result<Vec<(Geometry, Vec<bool>)>, OvertureMapsCollectionError>{
        let places_records = self.collector.collect_from_release::<PlacesRecord>(
            self.release_version.clone(),
            self.places_row_filter_config.clone()
        )?;
        println!("Total places records {}", places_records.len());

        // Compute MEP category vectors
        let mep_vectors = map_taxonomy_model(
            self.places_taxonomy_model.clone(),
            places_records.iter().map(|record| record.get_categories().clone()).collect(),
            activity_types
        )?;

        println!("Total opportunities per category {:?}",
            (0..mep_vectors[0].len())
                .map(|i| mep_vectors.iter().map(|row| row[i] as i16 as f64).sum())
                .collect::<Vec<f64>>()
        );

        // Collect POI geometries
        let mep_geometries: Vec<Option<Geometry>> = places_records.into_par_iter()
                .map(|record| record.get_geometry())
                .collect();

        println!("Non-empty geometries: {:?}",
            mep_geometries.iter()
                .filter(|maybe_geometry| maybe_geometry.is_some())
                .collect::<Vec<_>>()
                .len()
        );

        // Zip geometries and vectors (Filtering Empty geometries in the process)
        Ok(mep_geometries.into_iter()
            .zip(mep_vectors)
            .filter_map(|(maybe_geometry, vector)| {
                maybe_geometry.map(|geometry| (geometry, vector))
            })
            .collect::<Vec<(Geometry, Vec<bool>)>>()
        )
    }

    fn collect_building_opportunities(&self, activity_types: &[String]) -> Result<Vec<(Geometry, Vec<bool>)>, OvertureMapsCollectionError>{
        // Build the taxonomy model from the mapping by transforming the vectors into HashSets
        let buildings_taxonomy_model = TaxonomyModel::from_mapping(
            self.buildings_activity_mappings.clone()
                .into_iter()
                .map(|(key, vec)| (key, HashSet::from_iter(vec.into_iter())))
                .collect()
        );
        
        // Use the collector to retrieve buildings data
        let buildings_records = self.collector.collect_from_release::<BuildingsRecord>(
            self.release_version.clone(),
            self.buildings_row_filter_config.clone()
        )?;
        println!("Total buildings records {}", buildings_records.len());

        // Compute MEP category vectors
        let mep_vectors = map_taxonomy_model(
            self.places_taxonomy_model.clone(),
            buildings_records
                .iter()
                .filter_map(|record| record.get_class())
                .map(|class| vec![class])
                .collect(),
            activity_types
        )?;

        // Collect geometries
        let mep_geometries: Vec<Option<Geometry>> = buildings_records.par_iter()
                .map(|record| record.get_geometry())
                .collect();

        // Zip geometries and vectors (Filtering Empty geometries in the process)
        Ok(
            mep_geometries.into_iter()
                .zip(mep_vectors)
                .filter_map(|(maybe_geometry, vector)| {
                    maybe_geometry.map(|geometry| (geometry, vector))
                })
                .collect::<Vec<(Geometry, Vec<bool>)>>()
        )
    }
}

/// Takes a taxonomy model and transform the vector of
/// string labels (categories) into a vector of MEP opportunity
/// categories.
fn map_taxonomy_model(taxonomy_model: Arc<TaxonomyModel>,
                      categories: Vec<Vec<String>>,
                      group_labels: &[String]
                    ) -> Result<Vec<Vec<bool>>, OvertureMapsCollectionError>{

    categories.par_iter().map(|category_vec|
        Ok(
            taxonomy_model.clone()
                .reverse_map(category_vec, group_labels.to_vec())?
                // Reduce Vec<Vec<bool>> to Vec<bool> applying OR logic
                .into_iter()
                .reduce(|mut acc, v|{
                    acc.iter_mut()
                       .zip(v.iter())
                       .for_each(|(a,b)| *a |= b);
                    acc
                }).unwrap_or_default()
                // Map bool to f64 - it is easier to merge different datasets like this
                // TODO: Is this limiting in any capacity?
                // .into_par_iter()
                // .map(|v| v as i16 as f64)
                // .collect::<Vec<f64>>()
        )
    ).collect()
}