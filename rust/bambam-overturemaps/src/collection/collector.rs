use std::sync::Arc;
use rayon::prelude::*;
use std::time::Instant;
use object_store::path::Path;
use arrow::array::RecordBatch;
use arrow::json::WriterBuilder;
use serde::de::DeserializeOwned;
use arrow::json::writer::JsonArray;
use futures::stream::{self, StreamExt};
use object_store::{ObjectMeta, ObjectStore};
use parquet::arrow::async_reader::ParquetObjectReader;
use parquet::arrow::arrow_reader::ArrowPredicate;
use parquet::arrow::async_reader::ParquetRecordBatchStreamBuilder;

use super::record::RecordDataset;
use super::OvertureMapsCollectorConfig;
use super::RowFilter;
use super::RowFilterConfig;
use super::OvertureMapsCollectionError;
use super::ReleaseVersion;


#[allow(unused)]
#[derive(Debug)]
pub struct OvertureMapsCollector{
    obj_store: Arc<dyn ObjectStore>,
    batch_size: usize
}

impl TryFrom<OvertureMapsCollectorConfig> for OvertureMapsCollector {
    type Error = OvertureMapsCollectionError;

    fn try_from(value: OvertureMapsCollectorConfig) -> Result<Self, Self::Error> {
        value.build()
    }
}

#[allow(unused)]
impl OvertureMapsCollector{

    pub fn new(object_store: Arc<dyn ObjectStore>, batch_size: usize) -> Self{
        Self {
            obj_store: object_store,
            // row_filter_config: row_filter_config,
            batch_size: batch_size
        }
    }

    fn get_latest_release(&self) -> Result<String, OvertureMapsCollectionError>{
        Ok("2025-02-19.0".to_string())
    }

    pub fn collect_from_path<D: RecordDataset>(&self, path: Path, row_filter_config: Option<RowFilterConfig>) -> Result<Vec<D::Record>, OvertureMapsCollectionError>{
        let filemeta_stream =  self.obj_store.list(Some(&path));

        let io_runtime = tokio::runtime::Runtime::new().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| OvertureMapsCollectionError::TokioError(format!("failure creating async rust tokio runtime: {}", e)))?;

        // Collect all metadata to create streams, synchronously
        let meta_objects = runtime.block_on(filemeta_stream.collect::<Vec<_>>())
            .into_iter().collect::<Result<Vec<ObjectMeta>,_>>()
            .map_err(|e| OvertureMapsCollectionError::MetadataError(e.to_string()))?;

        // Prepare the filter predicates
        let row_filter = if let Some(row_filter_config) = &row_filter_config{
            Some(RowFilter::try_from(row_filter_config.clone())?)
        }else { None };

        // Instantiate Stream Builders
        let mut streams = vec![];
        for meta in meta_objects{
            println!("File Name: {}, Size: {}", meta.location, meta.size);

            // Parquet objects in charge of processing the incoming stream
            let mut reader = ParquetObjectReader::new(self.obj_store.clone(), meta.location).with_runtime(io_runtime.handle().clone());
            let builder = runtime.block_on(ParquetRecordBatchStreamBuilder::new(reader))
                .map_err(|e| OvertureMapsCollectionError::ArrowReaderError{ source: e })?;

            // Implement the required query filters
            // For this we need the scema of each file so we get that from the builder
            let parquet_metadata = builder.metadata().file_metadata();
            
            // Build Arrow filters from RowFilter enum
            let predicates: Vec<Box<dyn ArrowPredicate>> = 
                if let Some(filter) = &row_filter{
                    filter.build(parquet_metadata)?
                } else{ vec![] };

            let row_filter = parquet::arrow::arrow_reader::RowFilter::new(predicates);
            
            // Build stream object
            let stream: parquet::arrow::async_reader::ParquetRecordBatchStream<ParquetObjectReader> = builder
                .with_row_filter(row_filter)
                .with_batch_size(self.batch_size)
                .build()
                .map_err(|e| OvertureMapsCollectionError::ParquetRecordBatchStreamError{ source: e })?;

            streams.push(stream);
        }
        
        println!("Started collection");
        let start_collection = Instant::now();
        let result_vec = runtime.block_on(stream::iter(streams)
            .flatten_unordered(None)
            .collect::<Vec<_>>());
        println!("Collection time {:?}", start_collection.elapsed());
        
        let records: Vec<Vec<D::Record>> = result_vec.into_iter()
            .collect::<Result<Vec<RecordBatch>, _>>()
            .map_err(|e| OvertureMapsCollectionError::RecordBatchRetrievalError{ source: e })?
            .into_par_iter()
            .map(deserialize_batch::<D::Record>)
            .collect::<Result<Vec<_>, OvertureMapsCollectionError>>()?;
        println!("Deserialization time {:?}", start_collection.elapsed());

        // Flatten the collection
        let flatten_records = records
            .into_iter()
            .flatten()
            .collect();
        println!("Total time {:?}", start_collection.elapsed());
        Ok(flatten_records)

    }

    pub fn collect_from_release<D: RecordDataset>(&self, release: ReleaseVersion, row_filter_config: Option<RowFilterConfig>) -> Result<Vec<D::Record>, OvertureMapsCollectionError>{
        let release_str = match release{
            ReleaseVersion::Latest => self.get_latest_release()?,
            other => String::from(other)
        };
        let path = Path::from(D::format_url(release_str));
        self.collect_from_path::<D>(path, row_filter_config)
    }
}

/// Deserialize recordBatch into type T
fn deserialize_batch<T>(record_batch: RecordBatch) -> Result<Vec<T>, OvertureMapsCollectionError>
where
    T:DeserializeOwned
{
    // Arrow custom builder
    let builder = WriterBuilder::new().with_explicit_nulls(true);

    // Write the record batch out as json bytes
    let buf = Vec::new();
    let mut writer = builder.build::<_, JsonArray>(buf);
    writer.write(&record_batch).map_err(|e| OvertureMapsCollectionError::DeserializeError(e.to_string()))?;
    writer.finish().map_err(|e| OvertureMapsCollectionError::DeserializeError(e.to_string()))?;
    let json_data = writer.into_inner();

    // Parse the string using serde_json
    let deserialized_rows: Vec<T> = serde_json::from_slice(json_data.as_slice())
                                .map_err(|e| OvertureMapsCollectionError::DeserializeError(format!("Serde error: {e}")))?;
    Ok(deserialized_rows)
}
