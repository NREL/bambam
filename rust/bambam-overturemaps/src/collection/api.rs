use serde::{Deserialize, Serialize};
use super::{ OvertureMapsCollectorConfig, TaxonomyModelBuilder, RowFilterConfig};

#[derive(Debug, Serialize, Deserialize)]
struct OpportunityCollectionPluginConfig{
    collector_config: OvertureMapsCollectorConfig,
    runs: Vec<CollectionRunConfig>,
    taxonomy_builder: TaxonomyModelBuilder
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionRunConfig{
    data_type: DatasetType,
    row_filter: Option<RowFilterConfig>
}

#[derive(Debug, Serialize, Deserialize)]
enum DatasetType{
    Places,
    Buildings
}