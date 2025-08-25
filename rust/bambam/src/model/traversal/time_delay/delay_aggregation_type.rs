use routee_compass_core::model::unit::AsF64;
use serde::{Deserialize, Serialize};
use uom::{si::f64::Time, ConstZero};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DelayAggregationType {
    #[default]
    Sum,
    Mean,
    Median,
}

impl DelayAggregationType {
    pub fn apply(&self, values: Vec<Time>) -> Option<Time> {
        if values.is_empty() {
            return None;
        }
        if values.len() == 1 {
            return Some(values[0]);
        }
        use DelayAggregationType as A;
        let agg = match self {
            A::Sum => values.into_iter().sum(),
            A::Mean => {
                let sum: Time = values.iter().fold(Time::ZERO, |acc: Time, val: &Time| acc + *val);
                sum
            }
            A::Median => {
                let mid_idx = values.len() / 2;
                values[mid_idx]
            }
        };
        Some(agg)
    }
}
