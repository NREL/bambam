//! constructors for [`StateVariableConfig`] instances in multimodal routing.
use crate::model::state::LegIdx;
use routee_compass_core::model::{
    state::{CustomVariableConfig, InputFeature, StateVariableConfig},
    unit::{DistanceUnit, TimeUnit},
};
use uom::{
    si::f64::{Length, Time},
    ConstZero,
};

/// config value representing an empty LegIdx, Mode, or RouteId.
pub const EMPTY: CustomVariableConfig = CustomVariableConfig::SignedInteger { initial: -1 };

pub fn active_leg_input_feature() -> InputFeature {
    InputFeature::Custom {
        name: "ActiveLeg".to_string(),
        unit: "i64".to_string(),
    }
}

pub fn active_leg() -> StateVariableConfig {
    StateVariableConfig::Custom {
        custom_type: "ActiveLeg".to_string(),
        value: EMPTY,
        accumulator: false,
    }
}

/// creates configuration for mode state variables
pub fn leg_mode() -> StateVariableConfig {
    StateVariableConfig::Custom {
        custom_type: "Mode".to_string(),
        value: EMPTY,
        accumulator: false,
    }
}

/// creates configuration for distance state variables
pub fn multimodal_distance(output_unit: Option<DistanceUnit>) -> StateVariableConfig {
    StateVariableConfig::Distance {
        initial: Length::ZERO,
        accumulator: true,
        output_unit,
    }
}

/// creates configuration for time state variables
pub fn multimodal_time(output_unit: Option<TimeUnit>) -> StateVariableConfig {
    StateVariableConfig::Time {
        initial: Time::ZERO,
        accumulator: true,
        output_unit,
    }
}

/// creates configuration for route_id state variables
pub fn multimodal_route_id() -> StateVariableConfig {
    StateVariableConfig::Custom {
        custom_type: "RouteId".to_string(),
        value: EMPTY,
        accumulator: false,
    }
}
