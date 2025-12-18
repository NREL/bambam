use std::sync::Arc;

use super::{BoardingConstraintEngine, BoardingConstraintModel};

use routee_compass_core::model::{
    frontier::{FrontierModel, FrontierModelError, FrontierModelService},
    state::StateModel,
};

pub struct BoardingConstraintService {
    pub engine: Arc<BoardingConstraintEngine>,
}

impl BoardingConstraintService {
    pub fn new(engine: BoardingConstraintEngine) -> BoardingConstraintService {
        BoardingConstraintService {
            engine: Arc::new(engine),
        }
    }
}

impl FrontierModelService for BoardingConstraintService {
    fn build(
        &self,
        _query: &serde_json::Value,
        _state_model: Arc<StateModel>,
    ) -> Result<Arc<dyn FrontierModel>, FrontierModelError> {
        Ok(Arc::new(BoardingConstraintModel::new(self.engine.clone())))
    }
}
