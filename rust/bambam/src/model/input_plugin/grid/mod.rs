pub mod extent_format;
pub mod grid_input_plugin;
pub mod grid_input_plugin_builder;
mod grid_ops;
pub mod grid_type;
pub mod h3_grid;

pub use grid_ops::create_grid_row;

pub const EXTENT: &str = "extent";
pub const EXTENT_FORMAT: &str = "extent_format";
pub const GRID_TYPE: &str = "grid";
pub const GRID_ID: &str = "grid_id";
pub const ORIGIN_X: &str = "origin_x";
pub const ORIGIN_Y: &str = "origin_y";
pub const GEOMETRY: &str = "geometry";
pub const POPULATION: &str = "population";
pub const POPULATION_SOURCE: &str = "population_source";
