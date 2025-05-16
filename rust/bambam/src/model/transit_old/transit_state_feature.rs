use routee_compass_core::model::state::{CustomFeatureFormat, OutputFeature};

/// the index of the Gtfs archive in TransitTraversalModel::archives
pub fn transit_network_id() -> (String, OutputFeature) {
    (
        String::from("transit_network_id"),
        OutputFeature::Custom {
            r#type: String::from("identifier: 0 => Unassigned, _ => TripId"),
            unit: String::from("unsigned int"),
            format: CustomFeatureFormat::UnsignedInteger { initial: 0 },
            accumulator: false,
        },
    )
}

/// a number that can be used to look up a trip id in a gtfs archive
pub fn trip_id_enumeration() -> (String, OutputFeature) {
    (
        String::from("trip_id_enumeration"),
        OutputFeature::Custom {
            r#type: String::from("identifier: 0 => Unassigned, _ => TripId"),
            unit: String::from("unsigned int"),
            format: CustomFeatureFormat::UnsignedInteger { initial: 0 },
            accumulator: false,
        },
    )
}
