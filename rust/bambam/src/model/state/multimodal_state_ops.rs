use routee_compass_core::model::state::{StateModel, StateModelError, StateVariable};
use serde_json::json;

/// helper function for creating a descriptive error when attempting to apply
/// the multimodal traversal model on a state that has not activated it's first trip leg.
pub fn error_inactive_state_traversal(
    state: &[StateVariable],
    state_model: &StateModel,
) -> StateModelError {
    let next_json = state_model.serialize_state(state, false).unwrap_or_else(
        |e| json!({"message": "unable to serialize state!", "error": format!("{e}")}),
    );
    let next_string = serde_json::to_string_pretty(&next_json)
        .unwrap_or_else(|e| String::from("<unable to serialize state!>"));
    StateModelError::RuntimeError(format!(
        "attempting multimodal traversal with state that has no active leg: {next_string}"
    ))
}
