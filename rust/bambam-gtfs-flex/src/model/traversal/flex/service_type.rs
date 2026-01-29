use chrono::NaiveDateTime;
use routee_compass_core::model::{
    network::Edge,
    state::{StateModel, StateVariable},
    traversal::TraversalModelError,
};

/// this
pub struct GtfsFlexServiceTypeModel {}

impl GtfsFlexServiceTypeModel {
    /// True if the engine variant depends on a query-time start time argument
    pub fn requires_start_time(&self) -> bool {
        // match self {
        //     GtfsFlexServiceTypeModel::ServiceTypeTwoOrThree { .. } => true,
        //     GtfsFlexServiceTypeModel::ServiceTypeThree { .. } => true,
        //     _ => false,
        // }
        todo!("checks if we need start_time on this edge")
    }

    /// Apply the logic of traversing an edge for this service type.
    pub fn traverse_edge(
        &self,
        _edge: &Edge,
        _state: &mut Vec<StateVariable>,
        _state_model: &StateModel,
        _start_time: &NaiveDateTime,
    ) -> Result<(), TraversalModelError> {
        todo!("
            1. grab the Option<ZoneId> crate::model::feature::fieldname::SRC_ZONE_ID from the state (using state_model)
            2. get the Option<ZoneId> of this edge (todo: rescue the multimodal mapping tool from bambam here)
                - if it is None, we are done
            3. if zone ids match, check 'valid_trips' to determine if this is a valid destination ->
                - if it is, set crate::model::feature::fieldname::EDGE_IS_GTFS_FLEX_DESTINATION
        ")
    }
}

// /// the data backing this edge in the traversal model, which varies by service type.
// /// for more information, see the README.md for this crate.
// pub enum GtfsFlexServiceTypeModel {
//     /// In this service type, trips are assigned a src_zone_id when they board.
//     /// The trip may travel anywhere, but may only treat locations within this zone as destinations.
//     ServiceTypeOne {
//         /// for this edge, either the zone or None if the edge is not within a zone.
//         edge_zone: Option<ZoneId>,
//     },

//     /// In this service type, trips are assigned a src_zone_id and departure_time
//     /// when they board. The trip may travel anywhere, but may only treat particular
//     /// locations as destinations.
//     ServiceTypeTwoOrThree {
//         /// for this edge, either the zone or None if the edge is not within a zone.
//         edge_zone: Option<ZoneId>,

//         // /// a mapping from source zone and (optional) departure time to some set of
//         // /// destination zones
//         // valid_trips: HashMap<(ZoneId, Option<NaiveDateTime>), HashSet<ZoneId>>,
//     },
//     // /// In this service type, trips are assigned a src_zone_id and departure_time when
//     // /// they board. The trip may travel anywhere, but may only treat particular locations
//     // /// as destinations.
//     // ServiceTypeThree {
//     //     /// for each edge, at trip departure time, either their zone or
//     //     /// None if the edge is not within a zone.
//     //     departure_edge_zones: Box<[Option<ZoneId>]>,
//     //     /// for each edge, at trip arrival time, either their zone or
//     //     /// None if the edge is not within a zone.
//     //     arrival_edge_zones: Box<[Option<ZoneId>]>,
//     //     /// a mapping from source zone and (optional) departure time to some set of
//     //     /// destination zones
//     //     valid_trips: HashMap<(ZoneId, Option<NaiveDateTime>), HashSet<ZoneId>>,
//     // },
// }

// impl GtfsFlexServiceTypeModel {
//     /// True if the engine variant depends on a query-time start time argument
//     pub fn requires_start_time(&self) -> bool {
//         match self {
//             GtfsFlexServiceTypeModel::ServiceTypeTwoOrThree { .. } => true,
//             GtfsFlexServiceTypeModel::ServiceTypeThree { .. } => true,
//             _ => false,
//         }
//     }

//     /// Apply the logic of traversing an edge for this service type.
//     pub fn traverse_edge(
//         &self,
//         _edge: &Edge,
//         _state: &mut Vec<StateVariable>,
//         _state_model: &StateModel,
//         _start_time: Option<&NaiveDateTime>,
//     ) -> Result<(), TraversalModelError> {
//         match self {
//             GtfsFlexServiceTypeModel::ServiceTypeOne { edge_zones: _ } => {
//                 todo!("
//                     1. grab the Option<ZoneId> crate::model::feature::fieldname::SRC_ZONE_ID from the state (using state_model)
//                     2. get the Option<ZoneId> of this edge (todo: rescue the multimodal mapping tool from bambam here)
//                         - if it is None, we are done
//                     3. if zone ids match, this is a valid destination -> set crate::model::feature::fieldname::EDGE_IS_GTFS_FLEX_DESTINATION
//                 ")
//             }
//             GtfsFlexServiceTypeModel::ServiceTypeTwoOrThree {
//                 edge_zones: _,
//                 valid_trips: _,
//             } => {
//                 todo!("
//                     1. grab the Option<ZoneId> crate::model::feature::fieldname::SRC_ZONE_ID from the state (using state_model)
//                     2. get the Option<ZoneId> of this edge (todo: rescue the multimodal mapping tool from bambam here)
//                         - if it is None, we are done
//                     3. if zone ids match, check 'valid_trips' to determine if this is a valid destination ->
//                         - if it is, set crate::model::feature::fieldname::EDGE_IS_GTFS_FLEX_DESTINATION
//                 ")
//             }
//             GtfsFlexServiceTypeModel::ServiceTypeThree {
//                 departure_edge_zones: _,
//                 arrival_edge_zones: _,
//                 valid_trips: _,
//             } => {
//                 todo!("same logic as type 2 but also check if this edge is an arrival edge zone")
//             }
//         }
//     }
// }
