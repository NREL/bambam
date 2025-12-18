use std::sync::Arc;

use routee_compass_core::model::frontier::{
    FrontierModelBuilder, FrontierModelError, FrontierModelService,
};
use routee_compass_core::util::geo::PolygonalRTree;

use super::{BoardingConstraintConfig, BoardingConstraintEngine, BoardingConstraintService};

pub struct BoardingConstraintBuilder {}

impl FrontierModelBuilder for BoardingConstraintBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn FrontierModelService>, FrontierModelError> {
        let config: BoardingConstraintConfig = serde_json::from_value(parameters.clone())
            .map_err(|e| FrontierModelError::BuildError(e.to_string()))?;
        let rtree = PolygonalRTree::new(vec![]).map_err(FrontierModelError::BuildError)?;
        let engine = BoardingConstraintEngine::new(config, rtree);
        let service = BoardingConstraintService::new(engine);
        Ok(Arc::new(service))
    }
}
