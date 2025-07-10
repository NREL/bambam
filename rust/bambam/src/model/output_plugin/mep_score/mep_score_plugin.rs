use super::activity_frequencies::ActivityFrequencies;
use super::modal_intensity_model::ModalIntensityModel;
use crate::model::output_plugin::bambam_field as field;
use crate::model::output_plugin::isochrone::time_bin::TimeBin;
use crate::model::output_plugin::mep_score::{
    IntensityCategory, MepScorePluginConfig, WeightingFactors,
};
use crate::model::output_plugin::opportunity::OpportunityRecord;
use crate::model::output_plugin::opportunity::{opportunity_iterator, OpportunityFormat};
use itertools::Itertools;
use routee_compass::app::compass::CompassComponentError;
use routee_compass::app::{compass::CompassAppError, search::SearchAppResult};
use routee_compass::plugin::output::OutputPlugin;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::algorithm::search::SearchInstance;
use routee_compass_core::config::ConfigJsonExtensions;
use serde_json::json;
use std::collections::HashMap;

pub struct MepScorePlugin {
    pub modal_intensity_model: ModalIntensityModel,
    pub modal_weighting_factors: WeightingFactors,
    pub activity_frequencies: ActivityFrequencies,
    pub normalizing_activity: String,
}

impl TryFrom<&MepScorePluginConfig> for MepScorePlugin {
    fn try_from(value: &MepScorePluginConfig) -> Result<Self, Self::Error> {
        let modal_intensity_model = ModalIntensityModel::try_from(&value.modal_intensity_model)?;
        let activity_parameters = ActivityFrequencies::try_from(&value.activity_frequencies)?;
        Ok(MepScorePlugin {
            modal_intensity_model,
            modal_weighting_factors: value.modal_weighting_factors.clone(),
            activity_frequencies: activity_parameters,
            normalizing_activity: value.normalizing_activity.clone(),
        })
    }

    type Error = CompassComponentError;
}

impl OutputPlugin for MepScorePlugin {
    /// calculates a MEP score for each activity type for this row.
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        let (app_result, si) = match result {
            Ok((r, si)) => (r, si),
            Err(e) => return Ok(()),
        };

        let opportunity_format = field::get::opportunity_format(output)?;
        let activity_types = field::get::activity_types(output)?;
        let opp_totals = field::get::totals(output)?;
        let mode = field::get::mode(output)?;
        let mode_factors = self.modal_weighting_factors.get(&mode).ok_or_else(|| {
            OutputPluginError::OutputPluginFailed(format!(
                "missing weighting factors for mode {}",
                mode
            ))
        })?;

        // load opportunity iterator based on opportunity record type granularity
        let records = match opportunity_format {
            OpportunityFormat::Aggregate => {
                opportunity_iterator::new_aggregated(&output, &activity_types)
            }
            OpportunityFormat::Disaggregate => {
                opportunity_iterator::new_disaggregated(&output, &activity_types, si)
            }
        }?
        .collect::<Result<Vec<_>, _>>()?;

        // compute mep for each row
        for row in records.into_iter() {
            let opp_term = create_opportunity_term(
                &row,
                &self.activity_frequencies,
                &opp_totals,
                &self.normalizing_activity,
            )?;
            let decay_term =
                create_decay_term(&row, &mode, &self.modal_intensity_model, &mode_factors, si)?;
            let mep = opp_term * decay_term;
            write_mep_score(output, &row, mep)?;
        }

        Ok(())
    }
}

pub fn create_opportunity_term(
    row: &OpportunityRecord,
    activity_frequencies: &ActivityFrequencies,
    opportunity_totals: &HashMap<String, f64>,
    normalizing_activity: &str,
) -> Result<f64, OutputPluginError> {
    let act = row.get_activity_type();
    let count = row.get_count();
    let geom = row.get_geometry();
    let freq_norm = activity_frequencies.get_frequency_term(act, Some(geom))?;
    let nj = opportunity_totals.get(act).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "activity frequence missing for activity '{}'",
            act
        ))
    })?;
    if nj == &0.0 {
        return Err(OutputPluginError::OutputPluginFailed(format!(
            "invalid activity count total of 0 for category '{}'",
            act
        )));
    }
    let nstar = opportunity_totals
        .get(normalizing_activity)
        .ok_or_else(|| {
            OutputPluginError::OutputPluginFailed(format!(
                "activity frequence missing for normalizing activity '{}'",
                normalizing_activity
            ))
        })?;
    let opp_term = count * (nstar / nj) * freq_norm;
    Ok(opp_term)
}

pub fn create_decay_term(
    row: &OpportunityRecord,
    mode: &str,
    modal_intensity_model: &ModalIntensityModel,
    modal_weighting_factors: &HashMap<IntensityCategory, f64>,
    si: &SearchInstance,
) -> Result<f64, OutputPluginError> {
    let mut decay_accumulator = 0.0;
    for (cat, weight) in modal_weighting_factors.iter() {
        let intensity = modal_intensity_model.get_intensity_value(&mode, cat, &row, si)?;
        decay_accumulator += intensity * weight;
    }
    let decay_term = decay_accumulator.exp();
    Ok(decay_term)
}

/// writes this mep score for this opportunity row into the output. uses information
/// in the output row to determine where to write the score.
pub fn write_mep_score(
    output: &mut serde_json::Value,
    row: &OpportunityRecord,
    mep: f64,
) -> Result<(), OutputPluginError> {
    // insert parent, if needed
    let parent_path_values = row.get_json_path();
    let parent_path = parent_path_values.iter().map(|s| s.as_str()).collect_vec();
    field::insert_nested(output, &parent_path, field::MEP, json![{}], false)
        .map_err(|s| OutputPluginError::OutputPluginFailed(s))?;

    // insert mep value.
    let path = parent_path
        .iter()
        .chain(std::iter::once(&field::MEP))
        .cloned()
        .collect_vec();
    field::insert_nested(output, &path, row.get_activity_type(), json![mep], true)
        .map_err(|s| OutputPluginError::OutputPluginFailed(s))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::model::output_plugin::{
        bambam_field,
        isochrone::time_bin::TimeBin,
        mep_score::mep_score_plugin::write_mep_score,
        opportunity::{OpportunityOrientation, OpportunityRecord},
    };
    use geo::{point, polygon};
    use serde_json::json;

    #[test]
    fn test_write_score_agg() {
        let act = String::from("coding");
        let mep = 3.14159;
        let mut output = json![{}];
        let row = OpportunityRecord::Aggregate {
            activity_type: act.clone(),
            geometry: geo::Geometry::Polygon(polygon!()),
            time_bin: TimeBin::new(None, 10),
            count: 100.0,
        };
        write_mep_score(&mut output, &row, mep).expect("should not fail");
        let meps = output
            .get("mep")
            .expect("should have created the 'mep' key");
        let mep_json = meps
            .get(&act)
            .expect("should have created an entry for this activity type");
        let mep_f64 = mep_json.as_f64().expect("should be an f64");
        assert_eq!(mep_f64, mep, "value should be idempotent");
    }

    #[test]
    fn test_write_score_dis() {
        let act = String::from("coding");
        let mep = 3.14159;
        let opp_id = String::from("8");
        let mut output = json![{bambam_field::OPPORTUNITIES: {"8": {}}}];
        let row = OpportunityRecord::Disaggregate {
            id: opp_id.clone(),
            activity_type: act.clone(),
            opportunity_orientation: OpportunityOrientation::DestinationVertexOriented,
            geometry: geo::Geometry::Point(point! {x: 0.0, y: 0.0 }),
            state: vec![],
        };
        write_mep_score(&mut output, &row, mep).expect("should not fail");
        println!("{:?}", output);
        let opps = output
            .get(bambam_field::OPPORTUNITIES)
            .expect("invariant failed: opportunities key already existed");
        let opp_8 = opps
            .get("8")
            .expect("invariant failed: opportunities.8 key already existed");
        let mep_section = opp_8
            .get(bambam_field::MEP)
            .expect("should have created the 'mep' key");
        let mep_value = mep_section
            .get(&act)
            .expect("should have created an entry for this activity type");
        let mep_f64 = mep_value.as_f64().expect("should be an f64");
        assert_eq!(mep_f64, mep, "value should be idempotent");
    }
}
