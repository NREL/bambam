use super::Flex2Config;

use routee_compass_core::model::traversal::TraversalModelError;

pub struct Flex2Engine {}

impl TryFrom<Flex2Config> for Flex2Engine {
    type Error = TraversalModelError;

    fn try_from(_config: Flex2Config) -> Result<Self, Self::Error> {
        todo!()
    }
}
