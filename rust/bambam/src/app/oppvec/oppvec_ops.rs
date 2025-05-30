use itertools::Itertools;
use std::collections::HashMap;

/// builds hashmap from the column_mapping string argument for wide-format mapping source files.
///
/// mapping from column name to activity type as comma-delimited string of "col->acts" statements, where
/// "col" is the source column name, and "acts" is a hyphen-delminited non-empty list of target activity categories.
/// example: "CNS07->retail-jobs,CNS16->healthcare-jobs,CNS05->jobs"
pub fn create_column_mapping(string: &str) -> Result<HashMap<String, String>, String> {
    string
        .split(",")
        .map(|inner| {
            match inner.split("->").collect_vec().as_slice() {
                [col, acts] => {
                    // todo: we should split acts here by hyphens
                    Ok((col.to_string(), acts.to_string()))
                }
                _ => Err(format!(
                    "invalid mapping string '{}' must be in the format 'col->acts'",
                    inner
                )),
            }
        })
        .collect::<Result<HashMap<_, _>, String>>()
}
