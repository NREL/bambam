use parquet::arrow::arrow_reader::ArrowPredicate;
use arrow::{array::{Array, BooleanArray, StringArray}, error::ArrowError};

pub struct HasClassRowPredicate{
    projection_mask: parquet::arrow::ProjectionMask
}

impl HasClassRowPredicate{
    pub fn new(projection_mask: parquet::arrow::ProjectionMask) -> Self{
        Self{
            projection_mask
        }
    }
}

impl ArrowPredicate for HasClassRowPredicate {
    fn projection(&self) -> &parquet::arrow::ProjectionMask {
        &self.projection_mask
    }

    fn evaluate(&mut self, batch: arrow::array::RecordBatch) -> Result<arrow::array::BooleanArray, arrow::error::ArrowError> {
        let class_array = batch.column_by_name("class")
                     .ok_or(ArrowError::ParquetError(String::from("`class` column not found")))?
                     .as_any().downcast_ref::<StringArray>()
                     .ok_or(ArrowError::ParquetError(String::from("Cannot cast column `class` to StringArray type")))?;


        let boolean_values: Vec<bool> = (0..class_array.len())
            .into_iter()
            .map(|i| {
                !class_array.value(i).is_empty()
            }).collect();
        Ok(BooleanArray::from(boolean_values))
    }
}