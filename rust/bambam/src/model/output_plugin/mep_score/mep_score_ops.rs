use super::activity_parameters::ActivityParameters;
use super::modal_intensity_values::ModalIntensityValues;
use crate::model::output_plugin::isochrone::time_bin::TimeBin;
use crate::model::output_plugin::mep_output_field as field;
use itertools::Either;
use routee_compass::{
    app::search::SearchAppResult,
    plugin::{input::InputField, output::OutputPluginError},
};
use routee_compass_core::config::ConfigJsonExtensions;
use serde_json::json;
use std::collections::HashMap;

pub fn get_mode(output: &serde_json::Value) -> Result<String, OutputPluginError> {
    let request: &serde_json::Value = output.get("request").ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(String::from("request missing from output JSON"))
    })?;
    let mode = request
        .get_config_string(&String::from("mode"), &String::from("request"))
        .map_err(|e| {
            OutputPluginError::OutputPluginFailed(format!(
                "unable to find 'mode' on 'request': {}",
                e
            ))
        })?;
    Ok(mode)
}

pub fn get_time_bin(output: &serde_json::Value) -> Result<TimeBin, OutputPluginError> {
    let time_bin: TimeBin = output
        .get_config_serde(&String::from(field::TIME_BIN), &String::from("request"))
        .map_err(|e| {
            OutputPluginError::OutputPluginFailed(format!(
                "unable to find {} on 'response': {}",
                field::TIME_BIN,
                e
            ))
        })?;
    Ok(time_bin)
}

type GlobalOpps = HashMap<String, f64>;
type DestinationOpps = HashMap<String, HashMap<String, f64>>;
pub fn get_opportunities(
    output: &serde_json::Value,
) -> Result<Either<GlobalOpps, DestinationOpps>, OutputPluginError> {
    let opps = output
        .get_config_serde::<GlobalOpps>(&field::OPPORTUNITIES, &String::from("response"))
        .map(Either::Left)
        .or_else(|_| {
            output
                .get_config_serde::<DestinationOpps>(
                    &field::OPPORTUNITIES,
                    &String::from("response"),
                )
                .map(Either::Right)
        })
        .map_err(|_c| {
            OutputPluginError::InternalError(String::from(
                "opportunities does not match expected global or destination-oriented formats",
            ))
        })?;
    Ok(opps)
}

type AggOppsIter<'a> = Box<dyn Iterator<Item = (&'a String, Option<f64>)> + 'a>;
type OppsIter<'a> =
    Box<dyn Iterator<Item = (&'a String, Result<AggOppsIter<'a>, OutputPluginError>)> + 'a>;

fn opps_iter(json: &serde_json::Value) -> Result<AggOppsIter<'_>, OutputPluginError> {
    let opps_map = json.as_object().ok_or_else(|| {
        OutputPluginError::InternalError(format!(
            "{} value expected to be an object, was not",
            field::OPPORTUNITIES
        ))
    })?;
    let result = opps_map.iter().map(|(k, v)| (k, v.as_f64()));
    Ok(Box::new(result))
}

pub fn get_aggregate_opportunities_iter(
    output: &mut serde_json::Value,
) -> Result<AggOppsIter<'_>, OutputPluginError> {
    let opps_value = output.get(field::OPPORTUNITIES).ok_or_else(|| {
        OutputPluginError::MissingExpectedQueryField(InputField::Custom(
            field::OPPORTUNITIES.into(),
        ))
    })?;

    opps_iter(opps_value)
}

pub fn get_disaggregate_opportunities_iter(
    output: &mut serde_json::Value,
) -> Result<OppsIter<'_>, OutputPluginError> {
    let opps_value = output.get(field::OPPORTUNITIES).ok_or_else(|| {
        OutputPluginError::MissingExpectedQueryField(InputField::Custom(
            field::OPPORTUNITIES.into(),
        ))
    })?;

    let opps_map = opps_value.as_object().ok_or_else(|| {
        OutputPluginError::InternalError(format!(
            "{} value expected to be an object, was not",
            field::OPPORTUNITIES
        ))
    })?;
    let result = opps_map.iter().map(|(k, v)| {
        let inner = opps_iter(v);
        (k, inner)
    });

    Ok(Box::new(result))
}

pub fn get_intensity(
    intensities: &HashMap<String, HashMap<String, f64>>,
    mode: &String,
    category: &String,
) -> Result<f64, OutputPluginError> {
    let mode_tree = intensities.get(mode).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!("intensities missing mode {}", mode))
    })?;
    let intensity = mode_tree.get(category).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "intensities for mode {} missing intensity category {}",
            mode, category
        ))
    })?;
    Ok(*intensity)
}

pub fn compute_mep_from_aggregate_opportunities<'a>(
    iter: AggOppsIter<'a>,
    cell_id: Option<&'a String>,
    mode: &'a String,
    time_bin: &'a TimeBin,
    search_result: &'a SearchAppResult,
    modal_intensity_values: &'a ModalIntensityValues,
    activity_parameters: &'a ActivityParameters,
) -> Result<serde_json::Value, OutputPluginError> {
    let id_integer = match cell_id {
        Some(id_str) => {
            let result = id_str.parse::<usize>().map_err(|_e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "expected cell_id value to be an (unsigned integer), found {}",
                    id_str
                ))
            })?;
            Some(result)
        }
        None => None,
    };
    let intensity_vector =
        modal_intensity_values.get_intensity_vector(mode, Some(time_bin), None, search_result)?;
    let intensity_sum = intensity_vector.iter().sum();
    let result = iter
        .map(|(act, cnt)| {
            let count = cnt.unwrap_or_default();
            let freq = activity_parameters.get_frequency(act, id_integer, search_result)?;
            let mep = compute_mep_row(count, freq, intensity_sum);
            Ok((act.clone(), mep))
        })
        .collect::<Result<HashMap<String, f64>, OutputPluginError>>()?;
    Ok(json!(result))
}

pub fn compute_mep_from_opportunities<'a>(
    id: Option<usize>,
    opportunities: AggOppsIter<'a>,
    activity_parameters: &'a ActivityParameters,
    intensity_vector: &[f64],
    result: &'a SearchAppResult,
) -> Result<HashMap<String, f64>, OutputPluginError> {
    // let mut result: HashMap<String, f64> = HashMap::new();
    let intensity_sum = intensity_vector.iter().sum();
    let result = opportunities
        .map(|(act, cnt)| {
            let count = cnt.unwrap_or_default();
            let freq = activity_parameters.get_frequency(act, id, result)?;
            let mep = compute_mep_row(count, freq, intensity_sum);
            Ok((act.clone(), mep))
        })
        .collect::<Result<HashMap<String, f64>, OutputPluginError>>()?;
    Ok(result)
}

pub fn compute_mep_row(
    activity_count: f64,
    activity_frequency: f64,
    intensity_vector_sum: f64,
) -> f64 {
    activity_count * activity_frequency * f64::exp(intensity_vector_sum)
}
