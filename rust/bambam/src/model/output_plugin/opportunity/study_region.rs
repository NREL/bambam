use bamcensus_core::model::identifier::{Geoid, StateCode};
use serde::{Deserialize, Serialize};

/// Describes what study region bounds the activity dataset, as either
/// the entire nation, or, as a combination of census geoids.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StudyRegion {
    National,
    Census { geoids: Vec<String> },
}

impl StudyRegion {
    /// get the collection of [`Geoid`]s that describe this study region.
    pub fn get_geoids(&self) -> Result<Vec<Geoid>, String> {
        match self {
            StudyRegion::National => StateCode::ALL
                .iter()
                .map(|s| Geoid::try_from(s.to_fips_string().as_str()))
                .collect::<Result<Vec<_>, _>>(),
            StudyRegion::Census { geoids } => geoids
                .iter()
                .map(|s| Geoid::try_from(s.as_str()))
                .collect::<Result<Vec<_>, _>>(),
        }
    }
}
