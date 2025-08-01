//! # Opportunity
//!
//! The modules below provide modeling for activities and processing
//! search destinations into opportunities.

mod destination_opportunity;
mod opportunity_format;
pub mod opportunity_iterator;
pub mod opportunity_model;
pub mod opportunity_model_config;
mod opportunity_orientation;
pub mod opportunity_output_plugin;
pub mod opportunity_output_plugin_builder;
mod opportunity_record;
mod opportunity_row_id;
pub mod opportunity_source;
pub mod opportunity_spatial_row;
pub mod source;
pub mod study_region;

pub use destination_opportunity::DestinationOpportunity;
pub use opportunity_format::OpportunityFormat;
pub use opportunity_iterator::OpportunityIterator;
pub use opportunity_orientation::OpportunityOrientation;
pub use opportunity_record::OpportunityRecord;
pub use opportunity_row_id::OpportunityRowId;
