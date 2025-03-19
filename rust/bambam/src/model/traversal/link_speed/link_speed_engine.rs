use crate::model::traversal::time_delay::time_delay_lookup::TimeDelayLookup;
use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::model::traversal::TraversalModelError;
use routee_compass_core::model::traversal::TraversalModelService;
use std::sync::Arc;

pub struct LinkSpeedEngine {
    pub mode: String,
    pub underlying_service: Arc<dyn TraversalModelService>,
    pub departure_delay: Option<TimeDelayLookup>,
    pub arrival_delay: Option<TimeDelayLookup>,
}

impl LinkSpeedEngine {
    pub fn new(
        config: &serde_json::Value,
        underlying: Arc<dyn TraversalModelService>,
    ) -> Result<LinkSpeedEngine, TraversalModelError> {
        let parent_key = String::from("link speed traversal engine");
        let mode = config
            .get_config_string(&String::from("mode"), &parent_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        let departure_delay_option = config.get("departure_delay").map(TimeDelayLookup::try_from);
        let departure_delay = match departure_delay_option {
            Some(Err(e)) => Err(TraversalModelError::BuildError(e.to_string())),
            Some(Ok(delays)) => Ok(Some(delays)),
            None => Ok(None),
        }?;

        let arrival_delay_option = config.get("arrival_delay").map(TimeDelayLookup::try_from);
        let arrival_delay = match arrival_delay_option {
            Some(Err(e)) => Err(TraversalModelError::BuildError(e.to_string())),
            Some(Ok(delays)) => Ok(Some(delays)),
            None => Ok(None),
        }?;

        let engine = LinkSpeedEngine {
            mode,
            underlying_service: underlying.clone(),
            departure_delay,
            arrival_delay,
        };
        Ok(engine)
    }
}
