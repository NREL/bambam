use crate::model::traversal::time_delay::time_delay_lookup::TimeDelayLookup;
use routee_compass_core::{
    config::{CompassConfigurationField, ConfigJsonExtensions},
    model::{
        traversal::TraversalModelError,
        unit::{DistanceUnit, Speed, SpeedUnit, TimeUnit},
    },
};

pub struct FixedTraversalEngine {
    pub mode: String,
    pub speed: Speed,
    pub departure_delay: Option<TimeDelayLookup>,
    pub arrival_delay: Option<TimeDelayLookup>,
    pub distance_unit: DistanceUnit,
    pub time_unit: TimeUnit,
    pub speed_unit: SpeedUnit,
}

impl FixedTraversalEngine {
    pub fn new(params: &serde_json::Value) -> Result<FixedTraversalEngine, TraversalModelError> {
        let traversal_key = CompassConfigurationField::Traversal.to_string();
        let mode = params
            .get_config_string(&String::from("mode"), &traversal_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        let distance_unit = params
            .get_config_serde::<DistanceUnit>(&String::from("distance_unit"), &traversal_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let time_unit = params
            .get_config_serde::<TimeUnit>(&String::from("time_unit"), &traversal_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let speed_unit = params
            .get_config_serde::<SpeedUnit>(&String::from("speed_unit"), &traversal_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let speed = params
            .get_config_serde::<Speed>(&String::from("speed"), &traversal_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        let departure_delay_option = params.get("departure_delay").map(TimeDelayLookup::try_from);
        let departure_delay = match departure_delay_option {
            Some(Err(e)) => Err(TraversalModelError::BuildError(e.to_string())),
            Some(Ok(delays)) => Ok(Some(delays)),
            None => Ok(None),
        }?;

        let arrival_delay_option = params.get("arrival_delay").map(TimeDelayLookup::try_from);
        let arrival_delay = match arrival_delay_option {
            Some(Err(e)) => Err(TraversalModelError::BuildError(e.to_string())),
            Some(Ok(delays)) => Ok(Some(delays)),
            None => Ok(None),
        }?;

        let engine = FixedTraversalEngine {
            mode,
            speed,
            departure_delay,
            arrival_delay,
            distance_unit,
            time_unit,
            speed_unit,
        };
        Ok(engine)
    }
}
