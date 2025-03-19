use routee_compass_core::model::state::{CustomFeatureFormat, StateFeature};

/// the index of the Gtfs archive in TransitTraversalModel::archives
pub fn transit_network_id() -> (String, StateFeature) {
    (
        String::from("transit_network_id"),
        StateFeature::Custom {
            r#type: String::from("identifier: 0 => Unassigned, _ => TripId"),
            unit: String::from("unsigned int"),
            format: CustomFeatureFormat::UnsignedInteger { initial: 0 },
        },
    )
}

/// a number that can be used to look up a trip id in a gtfs archive
pub fn trip_id_enumeration() -> (String, StateFeature) {
    (
        String::from("trip_id_enumeration"),
        StateFeature::Custom {
            r#type: String::from("identifier: 0 => Unassigned, _ => TripId"),
            unit: String::from("unsigned int"),
            format: CustomFeatureFormat::UnsignedInteger { initial: 0 },
        },
    )
}
