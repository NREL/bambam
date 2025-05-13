use std::collections::HashSet;

use parquet::arrow::arrow_reader::ArrowPredicate;
use arrow::{array::{Array, BooleanArray, StringArray}, error::ArrowError};

pub struct HasClassInRowPredicate{
    classes: HashSet<String>,
    projection_mask: parquet::arrow::ProjectionMask
}

impl HasClassInRowPredicate{
    pub fn new(classes: HashSet<String>, projection_mask: parquet::arrow::ProjectionMask) -> Self{
        Self{
            classes,
            projection_mask
        }
    }
}

impl ArrowPredicate for HasClassInRowPredicate {
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
                self.classes.contains(class_array.value(i))
            }).collect();
        Ok(BooleanArray::from(boolean_values))
    }
}