use routee_compass::plugin::{input::InputField, output::OutputPluginError};
use std::hash::Hash;

/// gets a value from a JSON at some path.
/// to create the path argument, split a dot-delimited json path.
/// expects only paths through objects, not arrays, though that could
/// be added if needed.
///
/// # Arguments
///
/// * `obj` - JSON object to retrieve from
/// * `path` - path within object to retrieve value
///
/// # Example
///
/// ```ignore
/// // from "request.mep.jobs"
/// let path = ["request", "mep", "jobs"];
/// let result = get_value(&json, &path);
/// ```
///
/// # Returns
///
/// The value at the path, or an error
pub fn get_value<'a, K>(
    obj: &'a serde_json::Value,
    path: &[&K],
) -> Result<&'a serde_json::Value, OutputPluginError>
where
    K: ?Sized + Ord + Eq + Hash + ToString + serde_json::value::Index,
{
    let mut result: &serde_json::Value = obj;
    for k in path.iter() {
        match result.get(k) {
            None => {
                let path_str = path
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                return Err(OutputPluginError::MissingExpectedQueryField(
                    InputField::Custom(path_str.to_string()),
                ));
            }
            Some(v) => {
                result = v;
            }
        }
    }

    Ok(result)
}

/// gets a value from a JSON at some path.
/// to create the path argument, split a dot-delimited json path.
/// expects only paths through objects, not arrays, though that could
/// be added if needed.
///
/// # Arguments
///
/// * `obj` - JSON object to retrieve from
/// * `path` - path within object to retrieve value
///
/// # Example
///
/// ```ignore
/// // from "request.mep.jobs"
/// let path = ["request", "mep", "jobs"];
/// let result = get_value(&json, &path);
/// ```
///
/// # Returns
///
/// The value at the path, or an error
pub fn get_optional_value<'a, K>(
    obj: &'a serde_json::Value,
    path: &[&K],
) -> Option<&'a serde_json::Value>
where
    K: ?Sized + Ord + Eq + Hash + ToString + serde_json::value::Index,
{
    let mut result: &serde_json::Value = obj;
    for k in path.iter() {
        match result.get(k) {
            None => {
                return None;
            }
            Some(v) => {
                result = v;
            }
        }
    }

    Some(result)
}

pub fn get_map_kvs<'a, K>(
    obj: &'a serde_json::Value,
    key: &K,
) -> Result<Vec<(&'a String, &'a serde_json::Value)>, OutputPluginError>
where
    K: ?Sized + Ord + Eq + Hash + ToString + serde_json::value::Index,
{
    let nested = obj.get(key).ok_or_else(|| {
        OutputPluginError::MissingExpectedQueryField(InputField::Custom(key.to_string()))
    })?;
    let nested_map = nested.as_object().ok_or_else(|| {
        OutputPluginError::InternalError(format!(
            "value at {} expected to be map, found {}",
            key.to_string(),
            nested
        ))
    })?;
    let nested_vec = nested_map.iter().collect::<Vec<_>>();
    Ok(nested_vec)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_get_nested_value() {
        let input = serde_json::json!({
            "request": {
                "hex_id": "123",
            },
            "ignored": true
        });
        let result = super::get_value(&input, &["request", "hex_id"])
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(result, "123");
    }
}
