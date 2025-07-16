pub mod activity_frequencies;
mod activity_frequencies_config;
mod intensity_category;
mod intensity_value;
mod intensity_value_config;
pub mod mep_score_plugin;
pub mod mep_score_plugin_builder;
mod mep_score_plugin_config;
mod modal_intensity_config;
pub mod modal_intensity_model;
use std::collections::HashMap;

pub use activity_frequencies_config::ActivityFrequenciesConfig;
pub use intensity_category::IntensityCategory;
pub use intensity_value::IntensityValue;
pub use intensity_value_config::IntensityValueConfig;
pub use mep_score_plugin_config::MepScorePluginConfig;
pub use modal_intensity_config::ModalIntensityConfig;

pub type ModeName = String;
pub type ModeIntensities = HashMap<IntensityCategory, IntensityValue>;
pub type ModeIntensitiesConfig = HashMap<IntensityCategory, IntensityValueConfig>;
/// for a given travel mode, for each modal weighting factor (time, energy, cost, etc),
/// an intensity value per passenger mile
pub type Intensities = HashMap<ModeName, ModeIntensities>;
pub type IntensitiesConfig = HashMap<ModeName, ModeIntensitiesConfig>;

/// factors to multiply against modal intensities
pub type WeightingFactors = HashMap<ModeName, HashMap<IntensityCategory, f64>>;
