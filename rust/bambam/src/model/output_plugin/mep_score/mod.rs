pub mod activity_parameters;
mod activity_parameters_config;
pub mod mep_score_ops;
pub mod mep_score_output_plugin;
pub mod mep_score_plugin_builder;
mod modal_intensity;
mod modal_intensity_config;
pub mod modal_intensity_model;
mod opportunity_instance;
mod spatial_coefficients;
mod spatial_intensities;

use std::collections::HashMap;

pub use activity_parameters_config::ActivityParametersConfig;
pub use modal_intensity::ModalIntensity;
pub use modal_intensity_config::ModalIntensityConfig;
pub use opportunity_instance::OpportunityAccessRecord;
pub use spatial_coefficients::SpatialCoefficients;
pub use spatial_intensities::SpatialIntensities;

/// for a given travel mode, for each modal weighting factor (time, energy, cost, etc),
/// an intensity value per passenger mile
pub type Intensities = HashMap<String, HashMap<String, ModalIntensity>>;

/// factors to multiply against modal intensities
pub type Coefficients = HashMap<String, f64>;
