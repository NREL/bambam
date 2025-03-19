use super::opportunity_format::OpportunityCollectFormat;
use super::opportunity_model::OpportunityModel;
use super::opportunity_model_config::OpportunityModelConfig;
use crate::model::output_plugin::{mep_output_field as field, mep_output_ops};
use routee_compass::app::{compass::CompassAppError, search::SearchAppResult};
use routee_compass::plugin::output::OutputPlugin;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::algorithm::search::SearchInstance;
use serde_json::json;

/// RouteE Compass output plugin that appends opportunities to a search result row.
/// uses the loaded [`OpportunityModel`] to look up points-of-interest and returns
/// appends these results either aggregated or disaggregate, based on the chosen
/// [`OpportunityCollectFormat`]. this is run for each expected [`TimeBin`] in the search
/// row.
pub struct OpportunityOutputPlugin {
    pub model: OpportunityModel,
    pub opportunity_format: OpportunityCollectFormat,
}

impl OutputPlugin for OpportunityOutputPlugin {
    /// tags a result with opportunity counts
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        // write down info about this opportunity format
        output[field::OPPORTUNITY_FORMAT] = json![self.opportunity_format.to_string()];
        // we use only destinations that changed from the last time bin, so we do "walk"
        // the previous TimeBin.min_time during iteration

        let walk_time_bin = true;
        let bin_iter = field::time_bins_iter_mut(output, walk_time_bin)
            .map_err(OutputPluginError::OutputPluginFailed)?;
        for (k, v) in bin_iter {
            match (k, result) {
                (Ok(time_bin), Ok((result, instance))) => {
                    let destinations_iter = mep_output_ops::collect_destinations(
                        result,
                        Some(&time_bin),
                        &instance.state_model,
                    );
                    let destination_opportunities = self
                        .model
                        .batch_collect_opportunities(destinations_iter, instance)?;

                    let opportunities_json = self.opportunity_format.serialize_opportunities(
                        &destination_opportunities,
                        self.model.activity_types(),
                    )?;
                    v[field::OPPORTUNITIES] = opportunities_json;
                }
                _ => {
                    v[field::OPPORTUNITIES] = json!({});
                }
            }
        }

        Ok(())
    }
}

impl OpportunityOutputPlugin {
    pub fn new(
        config: &OpportunityModelConfig,
        output_format: OpportunityCollectFormat,
    ) -> Result<OpportunityOutputPlugin, OutputPluginError> {
        let model = config.build()?;
        let plugin = OpportunityOutputPlugin {
            model,
            opportunity_format: output_format,
        };
        Ok(plugin)
    }
}
