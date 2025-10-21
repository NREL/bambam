use crate::util::date_deserialization_ops::naive_date_to_str;
use chrono::NaiveDate;
use routee_compass_core::model::traversal::TraversalModelError;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn deserialize_date_mapping<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, HashMap<NaiveDate, NaiveDate>>, D::Error>
where
    D: Deserializer<'de>,
{
    let original_map = HashMap::<String, HashMap<String, String>>::deserialize(deserializer)?;

    // Convert inner maps to NaiveDate keys/values
    let mut out: HashMap<String, HashMap<NaiveDate, NaiveDate>> =
        HashMap::with_capacity(original_map.len());
    for (route_id, inner) in original_map {
        let mut parsed_inner = HashMap::with_capacity(inner.len());
        for (k_str, v_str) in inner {
            let k = naive_date_to_str(&k_str)
                .map_err(|e| D::Error::custom(format!("failed to deserialize date mapping for route_id `{route_id}`: invalid date key `{k_str}`: {e}")))?;
            let v = naive_date_to_str(&v_str)
                .map_err(|e| D::Error::custom(format!("failed to deserialize date mapping for route_id `{route_id}`: invalid date value `{v_str}`: {e}")))?;
            parsed_inner.insert(k, v);
        }
        out.insert(route_id, parsed_inner);
    }

    Ok(out)
}

// We can add more to this as needed
#[derive(Serialize, Deserialize)]
pub struct GtfsArchiveMetadata {
    /// List of unique (fully-qualified) route identifiers used in the schedules
    pub fq_route_ids: Vec<String>,
    /// Mapping from target date to available date for each route_id
    #[serde(deserialize_with = "deserialize_date_mapping")]
    pub date_mapping: HashMap<String, HashMap<NaiveDate, NaiveDate>>,
}
