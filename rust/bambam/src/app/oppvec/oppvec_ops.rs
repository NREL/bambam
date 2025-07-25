use itertools::Itertools;
use std::collections::HashMap;

/// builds hashmap from the column_mapping string argument for wide-format mapping source files.
///
/// mapping from column name to activity type as comma-delimited string of "col->acts" statements, where
/// "col" is the source column name, and "acts" is a hyphen-delminited non-empty list of target activity categories.
/// example: "CNS07->retail-jobs,CNS16->healthcare-jobs,CNS05->jobs"
pub fn create_mapping(string: &str) -> Result<HashMap<String, Vec<String>>, String> {
    string
        .split(",")
        .map(|inner| {
            match inner.split("->").collect_vec().as_slice() {
                [col] => {
                    // user provided no mapping, we use the expected upstream
                    // activity category as the output activity name
                    Ok((col.to_string(), vec![col.to_string()]))
                }
                [col, acts] => {
                    let activity_categories = acts.split("-").map(|s| s.to_string()).collect_vec();
                    Ok((col.to_string(), activity_categories))
                }
                _ => Err(format!(
                    "invalid mapping string '{inner}' must be in the format 'col->acts'"
                )),
            }
        })
        .collect::<Result<HashMap<_, _>, String>>()
}
