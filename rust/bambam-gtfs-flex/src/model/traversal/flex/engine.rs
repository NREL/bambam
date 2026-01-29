
use chrono::{NaiveDateTime, NaiveTime, Timelike};
use routee_compass_core::{
    model::{
        network::Edge,
        state::{StateModel, StateVariable},
        traversal::TraversalModelError,
    },
    util::geo::PolygonalRTree,
};

use crate::model::{
    feature,
    traversal::flex::{
        zonal_relation::ZonalRelation, GtfsFlexTraversalConfig,
    },
};

/// the state of the engine may change at query time in the case of
/// service type 4. this is effectively a wrapper type for service
/// types 1-3.
///
/// see rust/bambam-gtfs-flex/README.md for more details on service types.
pub enum GtfsFlexTraversalEngine {
    /// a lookup for zones by vertex POINT geometry.
    ZonalModel {
        zones: PolygonalRTree<f32, ZonalRelation>,
    },
    /// for traversal in GTFS-Flex Service Types 1-3, we store an optional
    /// service type model for each edge in the edge list.
    // GtfsFlexModelsByEdgeId(Arc<Box<[Option<GtfsFlexServiceTypeModel>]>>),
    /// logic for traversal in GTFS-Flex Service Type 4. instead of using the road network,
    /// this model uses the edges generated between stops via the same logic as
    /// bambam-gtfs.
    /// in order to simulate pooling delays, the model uses a collection of sampled
    /// link travel time delays, which must be generated at query time.
    TypeFourWithDelays {
        /// delays sampled for each link in this system. these should be
        /// sampled at model instantiation time (Service::build()) but
        /// should be idempotent throughout the graph search for search
        /// correctness. if no delays are to be assigned, this can be None.
        delays: Option<Box<[Option<NaiveTime>]>>,
    },
}

impl TryFrom<&GtfsFlexTraversalConfig> for GtfsFlexTraversalEngine {
    type Error = TraversalModelError;

    fn try_from(_value: &GtfsFlexTraversalConfig) -> Result<Self, Self::Error> {
        todo!("read archive and produce one of the GtfsFlexTraversalEngine variants based on the result")
    }
}

impl GtfsFlexTraversalEngine {
    // /// true if the engine variant depends on a query-time start time argument
    // pub fn requires_start_time(&self) -> bool {
    //     match self {
    //         GtfsFlexTraversalEngine::GtfsFlexModelsByEdgeId(engine) => engine.requires_start_time(),
    //         _ => false,
    //     }
    // }

    /// apply the logic for traversing edges in GTFS-Flex based on the Service Type of this agency.
    pub fn traverse_edge(
        &self,
        edge: &Edge,
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
        _start_time: &NaiveDateTime,
    ) -> Result<(), TraversalModelError> {
        match self {
            GtfsFlexTraversalEngine::ZonalModel { zones: _ } => {
                todo!(
                    "
                    1. grab the dst vertex
                    2. get iterator of all intersecting zones
                    3. test accept.. <waiting for Sailesh, do we have AND logic or OR logic on matching here?>
                "
                )
                // let service_type = service_types.get(edge.edge_id.0).ok_or_else(|| {
                //     let msg = format!(
                //         "GTFS-Flex service types have no entry for edge list id, edge id: ({},{})",
                //         edge.edge_list_id, edge.edge_id
                //     );
                //     TraversalModelError::TraversalModelFailure(msg)
                // })?;
                // if let Some(st) = service_type {
                //     st.traverse_edge(edge, state, state_model, start_time)?;
                // }
                // Ok(())
            }

            GtfsFlexTraversalEngine::TypeFourWithDelays { delays } => {
                // check for a delay entry in our sampled delay dataset
                let delay_entry = match delays {
                    Some(ds) => {
                        ds.get(edge.edge_id.0).ok_or_else(|| {
                            let msg = format!("while applying gtfs service type 4 delay, found delays vector is out of index for edge {}", edge.edge_id);
                            TraversalModelError::TraversalModelFailure(msg)
                        }).cloned()
                    },
                    None => Ok(None),
                }?;

                // apply sampled pooling delay to state vector if present
                if let Some(delay) = delay_entry {
                    let delay_uom = uom::si::f64::Time::new::<uom::si::time::second>(
                        delay.num_seconds_from_midnight() as f64,
                    );
                    state_model.set_time(
                        state,
                        feature::fieldname::EDGE_POOLING_DELAY,
                        &delay_uom,
                    )?;
                }

                Ok(())
            }
        }
    }
}
