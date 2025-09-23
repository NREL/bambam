//! builds labels that include enumerations for leg modes.
//!
use routee_compass_core::model::{
    label::{label_model_error::LabelModelError, Label, LabelModel},
    network::VertexId,
    state::{StateModel, StateVariable},
};

use crate::model::state::{
    multimodal_state_ops as ops, LegIdx, MultimodalMapping, MultimodalStateMapping,
};

pub struct MultimodalLabelModel {
    mode_to_state: MultimodalStateMapping,
    max_trip_legs: LegIdx,
}

impl MultimodalLabelModel {
    pub fn new(
        mode_to_state: MultimodalStateMapping,
        max_trip_legs: LegIdx,
    ) -> MultimodalLabelModel {
        MultimodalLabelModel {
            mode_to_state,
            max_trip_legs,
        }
    }

    pub const ERR_EMPTY: &str = "cannot built a multimodal search Label for a trip with no legs";
}

impl LabelModel for MultimodalLabelModel {
    fn label_from_state(
        &self,
        vertex_id: VertexId,
        state: &[StateVariable],
        state_model: &StateModel,
    ) -> Result<Label, LabelModelError> {
        let mode_labels: Vec<u8> =
            ops::get_mode_label_sequence(state, state_model, self.max_trip_legs)?
                .into_iter()
                .map(|mode_label| {
                    mode_label.try_into().map_err(|e| {
                        LabelModelError::LabelModelError(format!(
                            "mode label {mode_label} cannot be expressed as u8: {e}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

        if mode_labels.is_empty() {
            Err(LabelModelError::LabelModelError(
                Self::ERR_EMPTY.to_string(),
            ))
        } else {
            let label = Label::new_u8_state(vertex_id, &mode_labels)?;
            Ok(label)
        }
    }
}

#[cfg(test)]
mod test {
    use routee_compass_core::model::access::AccessModel;
    use routee_compass_core::model::state::StateVariable;
    use routee_compass_core::model::{label::LabelModel, network::VertexId, state::StateModel};

    use crate::model::access::multimodal::MultimodalAccessModel;
    use crate::model::label::multimodal::{
        multimodal_label_ops as label_ops, MultimodalLabelModel,
    };
    use crate::model::state::MultimodalMapping;
    use crate::model::state::{multimodal_state_ops as state_ops, MultimodalStateMapping};
    #[test]
    fn test_err_on_empty() {
        let access_model = MultimodalAccessModel::new_local("walk", 1, &["walk"], &[])
            .expect("test invariant failed");
        let state_model = StateModel::new(access_model.state_features());
        let state = state_model.initial_state().expect("test invariant failed");
        let vertex_id = VertexId(0);
        let model = MultimodalLabelModel::new(MultimodalMapping::empty(), 1);

        let result = model.label_from_state(vertex_id, &state, &state_model);

        match result {
            Err(e) => {
                let e_msg = format!("{}", e);
                assert_eq!(&e_msg, MultimodalLabelModel::ERR_EMPTY);
            }
            Ok(label) => panic!("test failed, should have error"),
        }
    }

    #[test]
    fn test_store_leg_sequence_in_label() {
        // SETUP: assign a state with sequence ["drive", "transit", "walk"]
        let max_trip_legs = 3;
        let am = MultimodalAccessModel::new_local(
            "drive",
            max_trip_legs,
            &["walk", "bike", "drive", "tnc", "transit"],
            &[],
        )
        .expect("test invariant failed");
        let sm = StateModel::new(am.state_features());
        let mut state = sm.initial_state().expect("test invariant failed");
        inject_trip_legs(
            &["drive", "transit", "walk"],
            &mut state,
            &sm,
            &am.mode_to_state,
            max_trip_legs,
        );

        let vertex_id = VertexId(0);
        let model = MultimodalLabelModel::new(MultimodalMapping::empty(), max_trip_legs);

        // TEST
        let label = model
            .label_from_state(vertex_id, &state, &sm)
            .expect("test failed");
        let result = label_ops::get_mode_sequence(&label, &am.mode_to_state).expect("test failed");
        assert_eq!(result, &["drive", "transit", "walk"]);
    }

    fn inject_trip_legs(
        legs: &[&str],
        state: &mut [StateVariable],
        state_model: &StateModel,
        mode_to_state: &MultimodalStateMapping,
        max_trip_legs: u64,
    ) {
        for (leg_idx, mode) in legs.iter().enumerate() {
            state_ops::set_leg_mode(state, leg_idx as u64, mode, &state_model, &mode_to_state)
                .expect("test invariant failed");
            state_ops::increment_active_leg_idx(state, &state_model, max_trip_legs)
                .expect("test invariant failed");
        }
    }
}
