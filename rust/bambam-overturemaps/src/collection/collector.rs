use arrow::array::RecordBatch;
use arrow::json::writer::JsonArray;
use arrow::json::WriterBuilder;
use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use object_store::{path::Path, ListResult, ObjectMeta, ObjectStore};
use parquet::arrow::arrow_reader::ArrowPredicate;
use parquet::arrow::async_reader::ParquetObjectReader;
use parquet::arrow::async_reader::ParquetRecordBatchStreamBuilder;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Instant;

use crate::collection::BuildingsRecord;
use crate::collection::PlacesRecord;

use super::record::OvertureRecord;
use super::record::OvertureRecordType;
use super::OvertureMapsCollectionError;
use super::OvertureMapsCollectorConfig;
use super::ReleaseVersion;
use super::RowFilter;
use super::RowFilterConfig;

/// Stores the initialized object store and allows to collect
/// records by `collect_from_release` and `collect_from_path`
#[derive(Debug)]
pub struct OvertureMapsCollector {
    obj_store: Arc<dyn ObjectStore>,
    batch_size: usize,
}

impl TryFrom<OvertureMapsCollectorConfig> for OvertureMapsCollector {
    type Error = OvertureMapsCollectionError;

    fn try_from(value: OvertureMapsCollectorConfig) -> Result<Self, Self::Error> {
        value.build()
    }
}

impl OvertureMapsCollector {
    pub fn new(object_store: Arc<dyn ObjectStore>, batch_size: usize) -> Self {
        Self {
            obj_store: object_store,
            batch_size,
        }
    }

    fn get_latest_release(&self) -> Result<String, OvertureMapsCollectionError> {
        // Get runtime to consume async functions
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                OvertureMapsCollectionError::TokioError(format!(
                    "failure creating async rust tokio runtime: {}",
                    e
                ))
            })?;

        // Get the folder names for all releases
        let common_path = Path::from("release/");
        let ListResult {
            common_prefixes, ..
        } = runtime
            .block_on(self.obj_store.list_with_delimiter(Some(&common_path)))
            .map_err(|e| {
                OvertureMapsCollectionError::ConnectionError(format!(
                    "Could not retrieve list of folders to get latest OvertureMaps release: {}",
                    e
                ))
            })?;

        // Process all common prefixes to find latest date
        let mut version_tuples: Vec<(NaiveDate, String)> = common_prefixes
            .iter()
            .filter_map(|p| {
                let clean_str = p.to_string().strip_prefix("release/")?.to_string();

                let mut string_parts = clean_str.split(".");
                let date_part = NaiveDate::parse_from_str(string_parts.next()?, "%Y-%m-%d").ok()?;

                Some((date_part, clean_str))
            })
            .collect();

        // Get the latest date from tuples
        version_tuples.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        version_tuples
            .pop()
            .ok_or(OvertureMapsCollectionError::ConnectionError(String::from(
                "No version tuples generated while getting latest version string",
            )))
            .map(|(_, v)| v)
    }

    pub fn collect_from_path(
        &self,
        path: Path,
        record_type: &OvertureRecordType,
        row_filter_config: Option<RowFilterConfig>,
    ) -> Result<Vec<OvertureRecord>, OvertureMapsCollectionError> {
        let filemeta_stream = self.obj_store.list(Some(&path));

        let io_runtime = tokio::runtime::Runtime::new()
            .map_err(|e| OvertureMapsCollectionError::TokioError(e.to_string()))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                OvertureMapsCollectionError::TokioError(format!(
                    "failure creating async rust tokio runtime: {}",
                    e
                ))
            })?;

        // Collect all metadata to create streams, synchronously
        let meta_objects = runtime
            .block_on(filemeta_stream.collect::<Vec<_>>())
            .into_iter()
            .collect::<Result<Vec<ObjectMeta>, _>>()
            .map_err(|e| OvertureMapsCollectionError::MetadataError(e.to_string()))?;

        // Prepare the filter predicates
        let row_filter = if let Some(row_filter_config) = &row_filter_config {
            Some(RowFilter::try_from(row_filter_config.clone())?)
        } else {
            None
        };

        // Instantiate Stream Builders
        let mut streams = vec![];
        for meta in meta_objects {
            log::debug!("File Name: {}, Size: {}", meta.location, meta.size);

            // Parquet objects in charge of processing the incoming stream
            let reader = ParquetObjectReader::new(self.obj_store.clone(), meta.location)
                .with_runtime(io_runtime.handle().clone());
            let builder = runtime
                .block_on(ParquetRecordBatchStreamBuilder::new(reader))
                .map_err(|e| OvertureMapsCollectionError::ArrowReaderError { source: e })?;

            // Implement the required query filters
            // For this we need the scema of each file so we get that from the builder
            let parquet_metadata = builder.metadata().file_metadata();

            // Build Arrow filters from RowFilter enum
            let predicates: Vec<Box<dyn ArrowPredicate>> = if let Some(filter) = &row_filter {
                filter.build(parquet_metadata)?
            } else {
                vec![]
            };

            let row_filter = parquet::arrow::arrow_reader::RowFilter::new(predicates);

            // Build stream object
            let stream: parquet::arrow::async_reader::ParquetRecordBatchStream<
                ParquetObjectReader,
            > = builder
                .with_row_filter(row_filter)
                .with_batch_size(self.batch_size)
                .build()
                .map_err(
                    |e| OvertureMapsCollectionError::ParquetRecordBatchStreamError { source: e },
                )?;

            streams.push(stream);
        }

        log::info!("Started collection");
        let start_collection = Instant::now();
        let result_vec = runtime.block_on(
            stream::iter(streams)
                .flatten_unordered(None)
                .collect::<Vec<_>>(),
        );
        log::info!("Collection time {:?}", start_collection.elapsed());

        // Unpack record batches
        let record_batches: Vec<RecordBatch> = result_vec
            .into_iter()
            .collect::<Result<Vec<RecordBatch>, _>>()
            .map_err(|e| OvertureMapsCollectionError::RecordBatchRetrievalError { source: e })?;

        // Deserialize batches into recor types
        let records: Vec<Vec<OvertureRecord>> = match record_type {
            OvertureRecordType::Places => record_batches
                .into_par_iter()
                .map(deserialize_batch::<PlacesRecord>)
                .map(|records_result| {
                    records_result
                        .map(|records| records.into_iter().map(OvertureRecord::Places).collect())
                })
                .collect::<Result<Vec<_>, OvertureMapsCollectionError>>()?,
            OvertureRecordType::Buildings => record_batches
                .into_par_iter()
                .map(deserialize_batch::<BuildingsRecord>)
                .map(|records_result| {
                    records_result
                        .map(|records| records.into_iter().map(OvertureRecord::Buildings).collect())
                })
                .collect::<Result<Vec<_>, OvertureMapsCollectionError>>()?,
        };
        log::info!("Deserialization time {:?}", start_collection.elapsed());

        // Flatten the collection
        let flatten_records = records.into_iter().flatten().collect();
        log::info!("Total time {:?}", start_collection.elapsed());
        Ok(flatten_records)
    }

    pub fn collect_from_release(
        &self,
        release: ReleaseVersion,
        record_type: &OvertureRecordType,
        row_filter_config: Option<RowFilterConfig>,
    ) -> Result<Vec<OvertureRecord>, OvertureMapsCollectionError> {
        let release_str = match release {
            ReleaseVersion::Latest => self.get_latest_release()?,
            other => String::from(other),
        };
        log::info!(
            "Collecting OvertureMaps records from release {}",
            release_str
        );
        let path = Path::from(record_type.format_url(release_str));
        self.collect_from_path(path, record_type, row_filter_config)
    }
}

/// Deserialize recordBatch into type T
fn deserialize_batch<T>(record_batch: RecordBatch) -> Result<Vec<T>, OvertureMapsCollectionError>
where
    T: DeserializeOwned,
{
    // Arrow custom builder
    let builder = WriterBuilder::new().with_explicit_nulls(true);

    // Write the record batch out as json bytes
    let buf = Vec::new();
    let mut writer = builder.build::<_, JsonArray>(buf);
    writer
        .write(&record_batch)
        .map_err(|e| OvertureMapsCollectionError::DeserializeError(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| OvertureMapsCollectionError::DeserializeError(e.to_string()))?;
    let json_data = writer.into_inner();

    // Parse the string using serde_json
    let deserialized_rows: Vec<T> = serde_json::from_slice(json_data.as_slice())
        .map_err(|e| OvertureMapsCollectionError::DeserializeError(format!("Serde error: {e}")))?;
    Ok(deserialized_rows)
}
