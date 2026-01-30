use crate::model::traversal::{flex::ZoneId, flex2::zone_graph::ZoneGraph};

use super::Flex2Config;

use routee_compass_core::{model::traversal::TraversalModelError, util::geo::PolygonalRTree};

pub struct Flex2Engine {
    pub graph: ZoneGraph,
    pub rtree: PolygonalRTree<f64, ZoneId>,
}

impl TryFrom<Flex2Config> for Flex2Engine {
    type Error = TraversalModelError;

    fn try_from(_config: Flex2Config) -> Result<Self, Self::Error> {
        todo!()
    }
}
