use super::TimeDelayLookup;
use crate::model::fieldname;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    state::{InputFeature, OutputFeature, StateModel, StateVariable},
    traversal::{TraversalModel, TraversalModelError, TraversalModelService},
    unit::{Distance, Time},
};
use std::sync::Arc;

/// assigns time delays for trips that have a delay from the start of their trip.
/// for within-trip delays assigned to beginning travel in a mode, use a delay
/// during mode switch instead (doesn't exist yet)
pub struct TripArrivalDelayModel(Arc<TimeDelayLookup>);

impl TripArrivalDelayModel {
    pub fn new(lookup: Arc<TimeDelayLookup>) -> TripArrivalDelayModel {
        TripArrivalDelayModel(lookup)
    }
}

impl TraversalModelService for TripArrivalDelayModel {
    fn build(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, TraversalModelError> {
        let model: Arc<dyn TraversalModel> = Arc::new(Self::new(self.0.clone()));
        Ok(model)
    }
}

impl TraversalModel for TripArrivalDelayModel {
    fn input_features(&self) -> Vec<(String, InputFeature)> {
        vec![]
    }

    fn output_features(&self) -> Vec<(String, OutputFeature)> {
        vec![(
            fieldname::TRIP_ARRIVAL_DELAY.to_string(),
            OutputFeature::Time {
                time_unit: self.0.config.time_unit,
                initial: Time::ZERO,
                accumulator: false,
            },
        )]
    }

    fn traverse_edge(
        &self,
        trajectory: (&Vertex, &Edge, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let (_, _, destination) = trajectory;
        add_delay_time(destination, state, state_model, self.0.clone())
    }

    fn estimate_traversal(
        &self,
        od: (&Vertex, &Vertex),
        state: &mut Vec<StateVariable>,
        state_model: &StateModel,
    ) -> Result<(), TraversalModelError> {
        let (_, destination) = od;
        add_delay_time(destination, state, state_model, self.0.clone())
    }
}

/// at the end of each edge, write down the arrival delay to use if this location is treated as a destination
fn add_delay_time(
    destination: &Vertex,
    state: &mut Vec<StateVariable>,
    state_model: &StateModel,
    lookup: Arc<TimeDelayLookup>,
) -> Result<(), TraversalModelError> {
    if let Some((delay, delay_unit)) = lookup.get_delay_for_vertex(destination) {
        state_model.set_time(state, fieldname::TRIP_ARRIVAL_DELAY, &delay, &delay_unit)?;
    }
    Ok(())
}
