use routee_compass_core::config::ConfigJsonExtensions;
use routee_compass_core::{
    model::traversal::TraversalModelError,
    model::unit::{DistanceUnit, TimeUnit},
};

pub struct ScheduleTraversalEngine {
    pub mode: String,
    pub distance_unit: DistanceUnit,
    pub time_unit: TimeUnit,
}

impl ScheduleTraversalEngine {
    // pub fn

    pub fn new(params: &serde_json::Value) -> Result<ScheduleTraversalEngine, TraversalModelError> {
        let parent_key = String::from("schedule traversal model");

        // 1. load the calendar policy
        //    - exact date and time, or, possibly something more relaxed
        //    - filters the GTFS
        // 2. load all of the GTFS archives
        //    - apply the calendar policy
        // 2. construct a searchable graph
        //    - we will have to append scheduled links to the road network graph
        //      and read/use a mapping file from the GTFS link indentifiers to the
        //      indices within the extended graph
        //    - ...but this requires the user to pre-process that data each time.
        //      if we could store multiple graphs, we could get around this problem...

        let mode = params
            .get_config_string(&String::from("mode"), &parent_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        let distance_unit = params
            .get_config_serde::<DistanceUnit>(&String::from("distance_unit"), &parent_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let time_unit = params
            .get_config_serde::<TimeUnit>(&String::from("time_unit"), &parent_key)
            .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // let speed_unit = params
        //     .get_config_serde::<SpeedUnit>(&String::from("speed_unit"), &traversal_key)
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // let speed = params
        //     .get_config_serde::<Speed>(&String::from("speed"), &traversal_key)
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // let base_unit_speed = speed_unit.convert(speed, BASE_SPEED_UNIT);

        // let access_model_params = params
        //     .get(String::from("access_model"))
        //     .ok_or_else(|| CompassConfigurationError::ExpectedFieldForComponent(
        //         String::from("access_model"),
        //         String::from("traversal_model"),
        //     ))
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        // let departure_agg = params
        //     .get_config_serde::<AccessAggregationType>(
        //         &String::from("departure_agg"),
        //         &traversal_key,
        //     )
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        // let arrival_agg = params
        //     .get_config_serde::<AccessAggregationType>(&String::from("arrival_agg"), &traversal_key)
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;

        // let access_model = TimeAccessModel::new(access_model_params)
        //     .map_err(|e| TraversalModelError::BuildError(e.to_string()))?;
        let engine = ScheduleTraversalEngine {
            mode,
            distance_unit,
            time_unit,
        };
        Ok(engine)
    }
}
