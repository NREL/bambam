use routee_compass_core::model::state::{CustomVariableConfig, StateVariableConfig};

/// the index of the Gtfs archive in TransitTraversalModel::archives
pub fn transit_network_id() -> (String, StateVariableConfig) {
    (
        String::from("transit_network_id"),
        StateVariableConfig::Custom {
            custom_type: String::from("GtfsTransitNetworkId"),
            value: CustomVariableConfig::UnsignedInteger { initial: 0 },
            accumulator: false,
        },
    )
}

/// a number that can be used to look up a trip id in a gtfs archive
pub fn trip_id_enumeration() -> (String, StateVariableConfig) {
    (
        String::from("trip_id_enumeration"),
        StateVariableConfig::Custom {
            custom_type: String::from("GtfsTripId"),
            value: CustomVariableConfig::UnsignedInteger { initial: 0 },
            accumulator: false,
        },
    )
}
