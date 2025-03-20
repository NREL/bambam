use routee_compass_core::model::unit::{AsF64, Time};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
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
                let (sum, cnt) = values
                    .into_iter()
                    .fold((0.0, 0.0), |(acc, cnt), v| (acc + v.as_f64(), cnt + 1.0));
                if cnt == 0.0 {
                    Time::ZERO
                } else {
                    Time::from(sum / cnt)
                }
            }
            A::Median => {
                let mid_idx = values.len() / 2;
                values[mid_idx]
            }
        };
        Some(agg)
    }
}
