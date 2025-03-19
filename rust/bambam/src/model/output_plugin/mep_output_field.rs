use super::isochrone::time_bin::TimeBin;
use serde_json::Value;

/// target structural additions to Compass response JSON:
/// {
///   "opportunity_format": {"aggregate"|"disaggregate"},
///   "bin": {
///     10: {
///       "isochrone": {},
///       "opportunity" {},
///       "mep": {},
///       "info": {},
///     }
///   }
/// }
///
pub const TIME_BIN: &str = "bin";
pub const INFO: &str = "info";
pub const ISOCHRONE: &str = "isochrone";
pub const ISOCHRONE_TREE_COUNT: &str = "isochrone_tree_count";
pub const OPPORTUNITIES: &str = "opportunities";
pub const OPPORTUNITY_FORMAT: &str = "opportunity_format";
pub const OPP_FMT_AGGREGATE: &str = "aggregate";
pub const OPP_FMT_DISAGGREGATE: &str = "disaggregate";

pub const MEP: &str = "mep";

fn field_error(fields: Vec<&str>) -> String {
    let path = fields.join(".");
    format!("expected path {} missing from output row", path)
}

fn type_error(fields: Vec<&str>, expected_type: String) -> String {
    let path = fields.join(".");
    format!("expected value at path {} to be {}", path, expected_type)
}

// pub fn get_time_bins_mut(
//     output: &mut serde_json::Value,
// ) -> Result<&mut Map<String, Value>, PluginError> {
//     let mut bins_value = output
//         .get_mut(TIME_BIN)
//         .ok_or_else(|| field_error(&output.clone(), vec![TIME_BIN]))?;
//     let mut bins = bins_value
//         .as_object_mut()
//         .ok_or_else(|| type_error(bins_value, vec![TIME_BIN], String::from("JSON object")))?;
//     Ok(bins)
// }

type TimeBinsIterMut<'a> = Box<dyn Iterator<Item = (Result<TimeBin, String>, &'a mut Value)> + 'a>;

pub fn time_bins_iter_mut(
    output: &mut serde_json::Value,
    walk_min_time: bool,
) -> Result<TimeBinsIterMut<'_>, String> {
    let mut prev_max: u64 = 0;
    let bins_value = output
        .get_mut(TIME_BIN)
        .ok_or_else(|| field_error(vec![TIME_BIN]))?;
    let bins = bins_value
        .as_object_mut()
        .ok_or_else(|| type_error(vec![TIME_BIN], String::from("JSON object")))?
        .iter_mut()
        .map(move |(k, v)| {
            let time_bin = k
                .parse::<u64>()
                .map(|max_time| {
                    let min_time = if walk_min_time { prev_max } else { 0 };
                    TimeBin { min_time, max_time }
                })
                .map_err(|_| format!("could not parse {} as (unsigned) integer", k));
            // slide the current max to become the next min (used if walk_min_time is true)
            let this_max_result = time_bin.as_ref().map(|t| t.max_time);
            if let Ok(this_max) = this_max_result {
                prev_max = this_max;
            }
            (time_bin, v)
        });
    Ok(Box::new(bins))
}

pub fn insert_nested(
    json: &mut Value,
    path: &[&str],
    key: &str,
    value: Value,
) -> Result<(), String> {
    let mut cursor = json;
    for k in path {
        match cursor.get_mut(k) {
            Some(child) => {
                cursor = child;
            }
            None => return Err(field_error(path.to_vec())),
        }
    }
    cursor[key] = value;
    Ok(())
}
