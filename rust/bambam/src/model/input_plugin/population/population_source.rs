use crate::{model::input_plugin::grid, util::polygonal_rtree::PolygonalRTree};
use bamsoda_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType};
use bamsoda_app::app::acs_tiger;
use bamsoda_core::model::identifier::{Geoid, GeoidType};
use geo::Geometry;
use itertools::Itertools;
use kdam::{tqdm, Bar, BarExt};
use routee_compass_core::config::ConfigJsonExtensions;
use std::collections::HashSet;
use wkt::TryFromWkt;

pub enum PopulationSource {
    UsCensusAcs {
        states: PolygonalRTree<Geoid>,
        acs_type: AcsType,
        acs_year: u64,
        acs_resolution: Option<GeoidType>,
        acs_categories: Option<Vec<String>>,
        api_token: Option<String>,
    },
}

impl PopulationSource {
    /// creates the population dataset that will be appended to JSON queries.
    ///
    /// # Arguments
    /// * `queries` - RouteE queries, each assumed to contain an additional GEOMETRY field
    ///
    /// # Result
    /// * a vector of relevant population data (geometry, population count) intersecting the incoming queries
    pub fn create_dataset(
        &self,
        // queries: &Vec<serde_json::Value>,
        extent: &Geometry,
    ) -> Result<Vec<(Geometry, f64)>, String> {
        match self {
            PopulationSource::UsCensusAcs {
                states,
                acs_type,
                acs_year,
                acs_resolution,
                acs_categories,
                api_token,
            } => {
                // find the list of US states (by GEOID) that intersect the incoming query dataset.
                // we will only request ACS data for those states.
                // let mut unique_state_geoids: HashSet<Geoid> = HashSet::new();
                let state_geoids = states
                    .intersection(extent)?
                    .map(|s| s.data.clone())
                    .collect::<HashSet<_>>();
                // let geoids_iter = tqdm!(
                //     queries.iter(),
                //     desc = "identify intersecting US states",
                //     total = queries.len(),
                //     animation = "fillup"
                // );
                // for row in geoids_iter {
                //     let wkt = row.get_config_string(&grid::GEOMETRY, &"").map_err(|e| {
                //         format!("failure reading `geometry` key on grid row: {}", e)
                //     })?;
                //     let geometry = TryFromWkt::try_from_wkt_str(&wkt)
                //         .map_err(|e| format!("failure parsing WKT geometry: {}", e))?;
                //     let intersecting = states.intersection(&geometry)?;
                //     for state in intersecting {
                //         unique_state_geoids.insert(state.data.clone());
                //     }
                // }
                // eprintln!();

                let acs_get_query = match acs_categories {
                    Some(cats) => cats.to_vec(),
                    None => vec![String::from("B01001_001E")],
                };

                let queries = state_geoids
                    .into_iter()
                    .map(|geoid| {
                        let acs_geoid_query: AcsGeoidQuery =
                            AcsGeoidQuery::new(Some(geoid.clone()), *acs_resolution).unwrap();

                        AcsApiQueryParams::new(
                            None,
                            *acs_year,
                            *acs_type,
                            acs_get_query.to_vec(),
                            acs_geoid_query,
                            api_token.clone(),
                        )
                    })
                    .collect_vec();

                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("failure creating async rust tokio runtime: {}", e))?;

                let future = acs_tiger::run_batch(&queries);
                let res = runtime
                    .block_on(future)
                    .map_err(|e| format!("failure downloading LODES data: {}", e))?;
                if !res.join_errors.is_empty() || !res.tiger_errors.is_empty() {
                    let msg = format!("failures downloading LODES data.\nTIGER ERRORS (top 5):\n  {}\nJOIN ERRORS (top 5):\n  {}",
                        res.tiger_errors.iter().take(5).join("\n  "),
                        res.join_errors.iter().take(5).join("\n  ")
                    );
                    return Err(msg);
                }

                // used by progress bar
                let n_groups = &res
                    .join_dataset
                    .iter()
                    .unique_by(|r| r.geoid.clone())
                    .collect_vec()
                    .len();

                // group population results by geometry.
                let chunk_iter = res
                    .join_dataset
                    .into_iter()
                    .chunk_by(|r| r.geometry.clone());

                // sum population counts by geometry
                let mut bar = Bar::builder()
                    .total(*n_groups)
                    .desc("proportioning population into grid")
                    .build()
                    .map_err(|e| format!("error building progress bar: {}", e))?;
                let mut result = vec![];
                for (geometry, grouped) in &chunk_iter {
                    let mut population_value = 0.0;
                    for row in grouped {
                        let count = row.acs_value.as_f64_safe()?;
                        population_value += count;
                    }
                    result.push((geometry, population_value));
                    let _ = bar.update(1);
                }
                eprintln!();
                Ok(result)
            }
        }
    }
}
