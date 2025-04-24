use serde::{Deserialize, Serialize};

use super::ObjectStoreSource;
use super::OvertureMapsCollector;
use super::OvertureMapsCollectionError;


#[derive(Debug, Serialize, Deserialize)]
pub struct OvertureMapsCollectorConfig{
    obj_store_type: ObjectStoreSource,
    // row_filter_config: Option<RowFilterConfig>,
    batch_size: usize
}

impl Default for OvertureMapsCollectorConfig{
    fn default() -> Self {
        Self { 
            obj_store_type: ObjectStoreSource::AmazonS3,
            // row_filter_config: None,
            batch_size: 4096 * 32 
        }
    }
}

impl OvertureMapsCollectorConfig {
    pub fn new(obj_store_type: ObjectStoreSource, batch_size: usize) -> Self{
        Self { 
               obj_store_type, 
            //    row_filter_config: Some(row_filter),
               batch_size 
            }
    }

    pub fn build(&self) -> Result<OvertureMapsCollector, OvertureMapsCollectionError>
    { 
        Ok(
            OvertureMapsCollector::new(
                self.obj_store_type.build()?, 
                // self.row_filter_config.clone(),
                self.batch_size
            )
        )
    }
}