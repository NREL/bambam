use crate::{
    algorithm::truncation::ComponentFilter,
    model::{osm::graph::osm_element_filter::ElementFilter, OsmCliError},
};
use serde::{Deserialize, Serialize};

/// defines behaviors for an OSM network import
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OsmImportConfiguration {
    pub component_filter: ComponentFilter,
    pub element_filter: ElementFilter,
    consolidation_threshold: Option<uom::si::f64::Length>,
    pub ignore_osm_parsing_errors: bool,
    pub truncate_by_edge: bool,
    pub simplify: bool,
    pub consolidate: bool,
    pub parallelize: bool,
    pub overwrite: bool,
}

impl Default for OsmImportConfiguration {
    fn default() -> Self {
        Self {
            component_filter: Default::default(),
            element_filter: Default::default(),
            consolidation_threshold: Some(Self::default_consolidation_threshold()),
            ignore_osm_parsing_errors: false,
            truncate_by_edge: true,
            simplify: true,
            consolidate: true,
            parallelize: true,
            overwrite: false,
        }
    }
}

impl OsmImportConfiguration {
    fn default_consolidation_threshold() -> uom::si::f64::Length {
        uom::si::f64::Length::new::<uom::si::length::meter>(15.0)
    }

    pub fn get_consolidation_threshold(&self) -> uom::si::f64::Length {
        self.consolidation_threshold
            .unwrap_or_else(Self::default_consolidation_threshold)
    }
}

impl TryFrom<&String> for OsmImportConfiguration {
    type Error = OsmCliError;

    fn try_from(f: &String) -> Result<Self, Self::Error> {
        if f.ends_with(".toml") {
            let s = std::fs::read_to_string(f).map_err(|e| {
                OsmCliError::ConfigurationError(format!("failure reading {f}: {e}"))
            })?;
            toml::from_str(&s)
                .map_err(|e| OsmCliError::ConfigurationError(format!("failure decoding {f}: {e}")))
        } else if f.ends_with(".json") {
            let s = std::fs::read_to_string(f).map_err(|e| {
                OsmCliError::ConfigurationError(format!("failure reading {f}: {e}"))
            })?;
            serde_json::from_str(&s)
                .map_err(|e| OsmCliError::ConfigurationError(format!("failure decoding {f}: {e}")))
        } else {
            Err(OsmCliError::ConfigurationError(format!(
                "unsupported file type: {f}"
            )))
        }
    }
}
