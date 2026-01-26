use std::collections::{HashMap, HashSet};

use chrono::NaiveDateTime;
use gtfs_structures::Gtfs;
use routee_compass_core::model::traversal::TraversalModelError;

use crate::model::traversal::flex::{GtfsFlexTraversalConfig, ZoneId};

/// the data backing this traversal model, which varies by service type.
/// for more information, see the README.md for this crate.
pub enum GtfsFlexTraversalEngine {
    /// In this service type, trips are assigned a src_zone_id when they board.
    /// The trip may travel anywhere, but may only treat locations within this zone as destinations.
    ServiceTypeOne {
        /// for each edge, either their zone or None if the edge is not within a zone.
        edge_zones: Box<[Option<ZoneId>]>,
    },

    /// In this service type, trips are assigned a src_zone_id and departure_time
    /// when they board. The trip may travel anywhere, but may only treat particular
    /// locations as destinations.
    ServiceTypeTwo {
        /// for each edge, either their zone or None if the edge is not within a zone.
        edge_zones: Box<[Option<ZoneId>]>,
        /// a mapping from source zone and (optional) departure time to some set of
        /// destination zones
        valid_trips: HashMap<(ZoneId, Option<NaiveDateTime>), HashSet<ZoneId>>,
    },

    /// In this service type, trips are assigned a src_zone_id and departure_time when
    /// they board. The trip may travel anywhere, but may only treat particular locations
    /// as destinations.
    ServiceTypeThree {
        /// for each edge, at trip departure time, either their zone or
        /// None if the edge is not within a zone.
        departure_edge_zones: Box<[Option<ZoneId>]>,
        /// for each edge, at trip arrival time, either their zone or
        /// None if the edge is not within a zone.
        arrival_edge_zones: Box<[Option<ZoneId>]>,
        /// a mapping from source zone and (optional) departure time to some set of
        /// destination zones
        valid_trips: HashMap<(ZoneId, Option<NaiveDateTime>), HashSet<ZoneId>>,
    },
    /// In this service type, we are actually running GTFS-style routing. However,
    /// we also need to modify some static weights based on the expected delays due
    /// to trip deviations. This weights should be modified during trip/model
    /// initialization but made fixed to ensure search correctness.
    ServiceTypeFour {
        /// the GTFS archive to use during traversal. this is a stub; we may want our own
        /// intermediate representation during routing for performance or generalizability.
        gtfs: Gtfs,
    },
}

impl TryFrom<&GtfsFlexTraversalConfig> for GtfsFlexTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(_value: &GtfsFlexTraversalConfig) -> Result<Self, Self::Error> {
        todo!("read archive and produce one of the GtfsFlexTraversalEngine variants based on the result")
    }
}

impl GtfsFlexTraversalEngine {
    /// true if the engine variant depends on a query-time start time argument
    pub fn requires_start_time(&self) -> bool {
        match self {
            GtfsFlexTraversalEngine::ServiceTypeTwo { .. } => true,
            GtfsFlexTraversalEngine::ServiceTypeThree { .. } => true,
            _ => false,
        }
    }
}
