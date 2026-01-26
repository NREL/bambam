use std::sync::Arc;

use chrono::NaiveTime;
use gtfs_structures::Gtfs;

use crate::model::traversal::flex::GtfsFlexTraversalEngine;

/// the state of the engine may change at query time in the case of
/// service type 4. this is effectively a wrapper type for service
/// types 1-3.
pub enum GtfsFlexModelState {
    EngineOnly(Arc<GtfsFlexTraversalEngine>),
    TypeFourWithDelays {
        /// the gtfs archive for routing
        gtfs: Arc<Gtfs>,
        /// delays sampled for each link in this system. these should be
        /// sampled at model instantiation time (Service::build()) but
        /// should be idempotent throughout the graph search for search
        /// correctness.
        delays: Box<[Option<NaiveTime>]>,
    },
}
