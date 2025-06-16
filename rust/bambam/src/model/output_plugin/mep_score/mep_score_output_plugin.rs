use std::collections::HashMap;

use super::activity_parameters::ActivityParameters;
use super::mep_score_ops as ops;
use super::modal_intensity_model::ModalIntensityModel;
use crate::model::output_plugin::mep_output_field as field;
use crate::model::output_plugin::opportunity::opportunity_format::OpportunityCollectFormat;
use routee_compass::app::{compass::CompassAppError, search::SearchAppResult};
use routee_compass::plugin::output::OutputPlugin;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::algorithm::search::SearchInstance;
use routee_compass_core::config::ConfigJsonExtensions;
use serde_json::json;

pub struct MepScoreOutputPlugin {
    pub modal_intensity_values: ModalIntensityModel,
    pub activity_parameters: ActivityParameters,
}

impl OutputPlugin for MepScoreOutputPlugin {
    /// calculates a MEP score for each activity type for this row.
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        let walk_time_bin: bool = true;
        let mode = ops::get_mode(output)?;
        let opp_fmt: OpportunityCollectFormat = output
            .get_config_serde(&field::OPPORTUNITY_FORMAT, &String::from("response"))
            .map_err(|e| {
                OutputPluginError::InternalError(format!(
                    "during mep score plugin, cannot decode {}: {}",
                    field::OPPORTUNITY_FORMAT,
                    e
                ))
            })?;
        let opp_totals: HashMap<String, f64> = output
            .get_config_serde(&field::OPPORTUNITY_TOTALS, &String::from("response"))
            .map_err(|e| {
                OutputPluginError::InternalError(format!(
                    "during mep score plugin, cannot decode {} from output row: {}",
                    field::OPPORTUNITY_TOTALS,
                    e
                ))
            })?;

        let bins_iter = field::time_bins_iter_mut(output, walk_time_bin)
            .map_err(OutputPluginError::OutputPluginFailed)?;
        for (time_bin_result, time_bin_json) in bins_iter {
            match (time_bin_result, result) {
                (Ok(time_bin), Ok((search_result, _))) => {
                    let mep_scores_json = match opp_fmt {
                        OpportunityCollectFormat::Aggregate => {
                            let iter = ops::get_aggregate_opportunities_iter(time_bin_json)?;
                            ops::compute_mep_from_aggregate_opportunities(
                                iter,
                                None,
                                &mode,
                                &time_bin,
                                search_result,
                                &self.modal_intensity_values,
                                &self.activity_parameters,
                            )
                        }
                        OpportunityCollectFormat::Disaggregate => {
                            let disaggregate_mep_scores =
                                ops::get_disaggregate_opportunities_iter(time_bin_json)?
                                    .map(|(id, opps_result)| match opps_result {
                                        Ok(opps) => ops::compute_mep_from_aggregate_opportunities(
                                            opps,
                                            Some(id),
                                            &mode,
                                            &time_bin,
                                            search_result,
                                            &self.modal_intensity_values,
                                            &self.activity_parameters,
                                        )
                                        .map(|meps| (id.clone(), meps)),
                                        Err(e) => Err(e),
                                    })
                                    .collect::<Result<serde_json::Map<_, _>, _>>()?;

                            Ok(json![disaggregate_mep_scores])
                        }
                    }?;

                    time_bin_json[field::MEP] = mep_scores_json;
                }
                _ => {
                    time_bin_json[field::MEP] = json!({});
                }
            }
        }
        Ok(())
    }
}
