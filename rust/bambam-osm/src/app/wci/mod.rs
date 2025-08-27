mod way_attributes_for_wci;
mod way_geometry_and_data;
mod wci_ops;

pub use way_attributes_for_wci::WayAttributesForWCI;
pub use way_geometry_and_data::WayGeometryData;
pub use wci_ops::process_wci;

pub const MAX_WCI_SCORE: i32 = 9;
