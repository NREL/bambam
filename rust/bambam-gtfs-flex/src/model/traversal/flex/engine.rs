use std::path::Path;

use kdam::BarBuilder;
use routee_compass_core::{model::traversal::TraversalModelError, util::fs::read_utils};

use crate::model::traversal::flex::{GtfsFlexTraversalConfig, ZoneId};

/// the data backing this traversal model, which varies by service type.
/// for more information, see the README.md for this crate.
pub enum GtfsFlexTraversalEngine {
    /// In this service type, trips are assigned a src_zone_id when they board.
    ServiceTypeOne {
        /// for each edge, either their zone or None if the edge is not within a zone.
        edge_zones: Box<[Option<ZoneId>]>,
    },
    ServiceTypeTwo {},
}

impl TryFrom<&GtfsFlexTraversalConfig> for GtfsFlexTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(value: &GtfsFlexTraversalConfig) -> Result<Self, Self::Error> {
        match value {
            GtfsFlexTraversalConfig::ServiceTypeOne {
                edge_zone_input_file,
            } => Self::new_type_one(edge_zone_input_file),
            GtfsFlexTraversalConfig::ServiceTypeTwo {
                zone_time_lookup_input_file: _,
            } => todo!(),
        }
    }
}

impl GtfsFlexTraversalEngine {
    /// builds a service type one engine
    pub fn new_type_one<T>(input_file: &T) -> Result<GtfsFlexTraversalEngine, TraversalModelError>
    where
        T: AsRef<Path>,
    {
        let bar_builder = BarBuilder::default().desc("gtfs flex service type one: edge zones");
        let edge_zones: Box<[Option<ZoneId>]> =
            read_utils::from_csv(input_file, false, Some(bar_builder), None).map_err(|e| {
                TraversalModelError::BuildError(format!(
                    "failure reading service type 1 edge zones file from file '{}': {e}",
                    input_file.as_ref().to_str().unwrap_or_default()
                ))
            })?;
        Ok(Self::ServiceTypeOne { edge_zones })
    }
}
