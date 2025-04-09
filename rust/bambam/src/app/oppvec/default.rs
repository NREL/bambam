use std::collections::HashMap;

pub const OVERTURE_MAPS_GEOMETRY: &str = "geometry";
pub const COSTAR_LATITUDE: &str = "latitude";
pub const COSTAR_LONGITUDE: &str = "longitude";
pub const OVERTURE_CATEGORY_FIELD: &str = "categories";
pub const COSTAR_PROPERTYTYPE_FIELD: &str = "propertytype";
pub const COSTAR_PROPERTYSUBTYPE_FIELD: &str = "propertysubtype";

pub fn costar_category_mapping(propertytype: &str, propertysubtype: &str) -> Option<String> {
    match (propertytype, propertysubtype) {
        ("Health Care", _) => Some(String::from("healthcare")),
        ("Sports & Entertainment", _) => Some(String::from("entertainment")),
        ("Retail", _) => Some(String::from("retail")),
        ("Specialty", _) => Some(String::from("services")),
        (_, "Fast Food") => Some(String::from("food")),
        (_, "Restaurant") => Some(String::from("food")),
        (_, "Bar") => Some(String::from("food")),
        _ => None,
    }
}
