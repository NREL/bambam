use std::collections::HashMap;

use super::opportunity_format::OpportunityFormat;
use super::opportunity_model::OpportunityModel;
use super::opportunity_model_config::OpportunityModelConfig;
use crate::model::output_plugin::{bambam_field as field, mep_output_ops};
use itertools::Itertools;
use routee_compass::app::{compass::CompassAppError, search::SearchAppResult};
use routee_compass::plugin::output::OutputPlugin;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::algorithm::search::SearchInstance;
use routee_compass_core::util::duration_extension::DurationExtension;
use serde_json::json;
use std::time::{Duration, Instant};

/// RouteE Compass output plugin that appends opportunities to a search result row.
/// uses the loaded [`OpportunityModel`] to look up points-of-interest and returns
/// appends these results either aggregated or disaggregate, based on the chosen
/// [`OpportunityCollectFormat`]. this is run for each expected [`TimeBin`] in the search
/// row.
pub struct OpportunityOutputPlugin {
    pub model: OpportunityModel,
    pub totals: HashMap<String, f64>,
    pub opportunity_format: OpportunityFormat,
}

impl OutputPlugin for OpportunityOutputPlugin {
    /// tags a result with opportunity counts
    fn process(
        &self,
        output: &mut serde_json::Value,
        result: &Result<(SearchAppResult, SearchInstance), CompassAppError>,
    ) -> Result<(), OutputPluginError> {
        let start_time = Instant::now();
        let (app_result, si) = match result {
            Ok((r, si)) => (r, si),
            Err(e) => {
                field::insert_nested_with_parents(
                    output,
                    &[field::INFO],
                    field::OPPORTUNITY_PLUGIN_RUNTIME,
                    json![Duration::ZERO.hhmmss()],
                    true,
                )
                .map_err(OutputPluginError::OutputPluginFailed)?;
                return Ok(());
            }
        };

        // write down model and global info
        output[field::OPPORTUNITY_FORMAT] = json![self.opportunity_format.to_string()];
        output[field::ACTIVITY_TYPES] = json![self.model.activity_types()];
        output[field::OPPORTUNITY_TOTALS] = json![self.totals];

        // we use only destinations that changed from the last time bin, so we do "walk"
        // the previous TimeBin.min_time during iteration
        match self.opportunity_format {
            OpportunityFormat::Aggregate => {
                process_aggregate_opportunities(output, app_result, si, self)?;
            }
            OpportunityFormat::Disaggregate => {
                process_disaggregate_opportunities(output, app_result, si, self)?;
            }
        }

        // write the plugin runtime
        let dur = Instant::now().duration_since(start_time);
        field::insert_nested_with_parents(
            output,
            &[field::INFO],
            field::OPPORTUNITY_PLUGIN_RUNTIME,
            json![dur.hhmmss()],
            false,
        )
        .map_err(OutputPluginError::OutputPluginFailed)?;
        println!("{}", serde_json::to_string(&output).unwrap());
        Ok(())
    }
}

impl OpportunityOutputPlugin {
    pub fn new(
        config: &OpportunityModelConfig,
        opportunity_format: OpportunityFormat,
    ) -> Result<OpportunityOutputPlugin, OutputPluginError> {
        let model = config.build()?;
        let totals = model.opportunity_totals().map_err(|e| {
            OutputPluginError::BuildFailed(format!("failed to collect opportunity totals: {}", e))
        })?;
        for (act, total) in totals.iter() {
            if total == &0.0 {
                return Err(OutputPluginError::BuildFailed(format!(
                    "opportunity totals for activity type {} are zero, which is invalid",
                    act
                )));
            }
        }
        let plugin = OpportunityOutputPlugin {
            model,
            totals,
            opportunity_format,
        };
        Ok(plugin)
    }
}

fn process_disaggregate_opportunities(
    output: &mut serde_json::Value,
    result: &SearchAppResult,
    instance: &SearchInstance,
    plugin: &OpportunityOutputPlugin,
) -> Result<(), OutputPluginError> {
    let destinations_iter =
        mep_output_ops::collect_destinations(result, None, &instance.state_model);
    let opps = plugin
        .model
        .collect_trip_opportunities(destinations_iter, instance)?;
    let opportunities_json = plugin
        .opportunity_format
        .serialize_opportunities(&opps, &plugin.model.activity_types())?;
    output[field::OPPORTUNITIES] = opportunities_json;
    Ok(())
}

/// for aggregate opportunity formats, we collect all opportunities within each time band
/// and bundle them together into a single output row.
fn process_aggregate_opportunities(
    output: &mut serde_json::Value,
    result: &SearchAppResult,
    instance: &SearchInstance,
    plugin: &OpportunityOutputPlugin,
) -> Result<(), OutputPluginError> {
    let bins = field::get_time_bins(output).map_err(OutputPluginError::OutputPluginFailed)?;

    for time_bin in bins {
        let start_time = Instant::now();

        // collect all opportunities from destinations within this time bin as a JSON object
        let destinations_iter =
            mep_output_ops::collect_destinations(result, Some(&time_bin), &instance.state_model);
        let destination_opportunities = plugin
            .model
            .collect_trip_opportunities(destinations_iter, instance)?;
        let opportunities_json = plugin
            .opportunity_format
            .serialize_opportunities(&destination_opportunities, &plugin.model.activity_types())?;

        // write opportunities
        let time_bin_key = time_bin.key();
        field::insert_nested_with_parents(
            output,
            &[field::TIME_BINS, &time_bin_key],
            field::OPPORTUNITIES,
            opportunities_json,
            false,
        )
        .map_err(OutputPluginError::OutputPluginFailed)?;

        // write runtime
        let runtime = Instant::now().duration_since(start_time);
        field::insert_nested_with_parents(
            output,
            &[field::TIME_BINS, &time_bin_key, field::INFO],
            field::OPPORTUNITY_BIN_RUNTIME,
            json![runtime.hhmmss()],
            false,
        )
        .map_err(OutputPluginError::OutputPluginFailed)?;
    }
    Ok(())
}
