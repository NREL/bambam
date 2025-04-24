use std::sync::Arc;
use parquet::arrow::ProjectionMask;
use parquet::file::metadata::FileMetaData;
use parquet::arrow::arrow_reader::ArrowPredicate;
use serde::{Deserialize, Serialize};

use super::bbox_row_predicate::{Bbox, BboxRowPredicate};
use super::has_class_row_predicate::HasClassRowPredicate;
use crate::collection::taxonomy::TaxonomyModel;
use crate::collection::error::OvertureMapsCollectionError;
use crate::collection::taxonomy::TaxonomyModelBuilder;
use crate::collection::filter::taxonomy_filter_predicate::TaxonomyRowPredicate;

#[allow(unused)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum RowFilterConfig{
    HasClass,
    Bbox{ xmin: f32, xmax: f32, ymin: f32, ymax: f32 },
    TaxonomyModel{ taxonomy_builder: TaxonomyModelBuilder },
    Combined {filters: Vec<Box<RowFilterConfig>>}
}

/// This enum holds all the data to build a predicate
/// except for the schema projection, which must be
/// supplied after reading file metadata
#[derive(Debug, Clone)]
pub enum RowFilter{
    HasClass,
    Bbox{ bbox: Bbox },
    TaxonomyModel{ taxonomy_model: TaxonomyModel },
    Combined { filters: Vec<Box<RowFilter>> }
}

impl TryFrom<RowFilterConfig> for RowFilter{
    type Error = OvertureMapsCollectionError;

    fn try_from(value: RowFilterConfig) -> Result<Self, OvertureMapsCollectionError> {
        use RowFilterConfig as C;
    
        match value {
            C::HasClass => Ok(Self::HasClass),
            C::Bbox { xmin, xmax, ymin, ymax } => {
                Ok(Self::Bbox { bbox: Bbox::new(xmin, xmax, ymin, ymax) })
            },
            C::TaxonomyModel { taxonomy_builder } => {
                Ok(Self::TaxonomyModel { taxonomy_model: taxonomy_builder.build()? })
            }
            C::Combined { filters } => {
                Ok(Self::Combined {
                    filters: 
                        filters
                        .into_iter().map(|f|{
                            Ok::<_, OvertureMapsCollectionError>(
                                Box::new(RowFilter::try_from(*f)?)
                            )
                        })
                        .collect::<Result<Vec<Box<RowFilter>>, OvertureMapsCollectionError>>()?
                })
            }
        }
    }
}


impl RowFilter{
    pub fn get_column_projection(&self) -> Vec<String>{
        use RowFilter as R;
        match self{
            R::HasClass => vec![String::from("class")],
            R::Bbox { .. } => vec![String::from("bbox")],
            R::TaxonomyModel { .. } => vec![String::from("categories")],
            R::Combined { .. } => vec![]
        }
    }

    pub fn build(&self, metadata: &FileMetaData) -> Result<Vec<Box<dyn ArrowPredicate>>, OvertureMapsCollectionError>{
        use RowFilter as R;
        let column_projection = self.get_column_projection();
        
        match self{
            R::HasClass => {
                Ok(vec![
                    Box::new(
                        HasClassRowPredicate::new(
                            ProjectionMask::columns(metadata.schema_descr(), column_projection.iter().map(|s| s.as_str()))
                        )
                    )
                ])
            }
            R::Bbox { bbox } => {
                Ok(vec![
                    Box::new(
                        BboxRowPredicate::new(
                            bbox.clone(),
                            ProjectionMask::columns(metadata.schema_descr(), column_projection.iter().map(|s| s.as_str()))
                        )
                )])
            },
            R::TaxonomyModel { taxonomy_model } => {
                Ok(vec![
                    Box::new(
                        TaxonomyRowPredicate::new(
                            Arc::new(taxonomy_model.clone()),
                            ProjectionMask::columns(metadata.schema_descr(), column_projection.iter().map(|s| s.as_str()))
                        )
                )])
            },
            R::Combined { filters } => {
                Ok(
                    filters.into_iter().map(|f|{
                        f.build(metadata)
                    })
                    .collect::<Result<Vec<Vec<Box<dyn ArrowPredicate>>>, OvertureMapsCollectionError>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<Box<dyn ArrowPredicate>>>()
                )
            }
        }
    }
}