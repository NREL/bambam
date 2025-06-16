use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::model::unit::{AsF64, DistanceUnit, EnergyUnit, TimeUnit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModalIntensity {
    /// intensity in energy per unit distance
    Energy {
        intensity: f64,
        unit: EnergyUnit,
        per_unit: DistanceUnit,
    },
    Dollar {
        intensity: f64,
        per_unit: DistanceUnit,
    },
}

impl ModalIntensity {
    pub fn get_intensity(&self, per_unit: &DistanceUnit) -> Result<f64, OutputPluginError> {
        match self {
            ModalIntensity::Energy {
                intensity,
                unit,
                per_unit,
            } => todo!(),
            ModalIntensity::Dollar {
                intensity,
                per_unit,
            } => {}
        }
    }
}
