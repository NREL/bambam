use super::{bambam_field, isochrone::time_bin::TimeBin};
use geo::{line_measures::LengthMeasurable, Haversine, InterpolatableLine, LineString, Point};
use routee_compass::{app::search::SearchAppResult, plugin::PluginError};
use routee_compass_core::{
    algorithm::search::SearchTreeBranch,
    model::{
        network::VertexId,
        state::{StateModel, StateModelError},
        unit::{AsF64, Convert, Distance, DistanceUnit},
    },
};
use std::{borrow::Cow, collections::HashMap};
use wkt::ToWkt;

pub type DestinationsIter<'a> =
    Box<dyn Iterator<Item = Result<(VertexId, &'a SearchTreeBranch), StateModelError>> + 'a>;

/// collects search tree branches that can be reached _as destinations_
/// within the given time bin.
pub fn collect_destinations<'a>(
    search_result: &'a SearchAppResult,
    time_bin: Option<&'a TimeBin>,
    state_model: &'a StateModel,
) -> DestinationsIter<'a> {
    match search_result.trees.first() {
        None => Box::new(std::iter::empty()),
        Some(tree) => {
            let tree_destinations = tree.iter().filter_map(move |(v_id, branch)| {
                let result_state = &branch.edge_traversal.result_state;
                let within_bin = match &time_bin {
                    Some(bin) => bin.state_time_within_bin(result_state, state_model),
                    None => Ok(true),
                };
                match within_bin {
                    Ok(true) => Some(Ok((*v_id, branch))),
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                }
            });

            Box::new(tree_destinations)
        }
    }
}

pub fn points_along_linestring(
    linestring: &LineString<f32>,
    stride: &Distance,
    distance_unit: &DistanceUnit,
) -> Result<Vec<Point<f32>>, String> {
    let mut stride_internal = Cow::Borrowed(stride);
    distance_unit
        .convert(&mut stride_internal, &DistanceUnit::Meters)
        .map_err(|e| e.to_string())?;
    let stride_f32 = stride_internal.as_f64() as f32;
    let line_length_meters = linestring.length(&Haversine);

    if line_length_meters < stride_f32 {
        match (linestring.points().next(), linestring.points().next_back()) {
            (Some(first), Some(last)) => Ok(vec![first, last]),
            _ => Err(format!(
                "invalid linestring, should have at least two points: {linestring:?}"
            )),
        }
    } else {
        // determine number of steps
        let n_strides = (line_length_meters / stride_f32).ceil();
        let n_strides_rounded = n_strides as i64;
        let n_points = n_strides_rounded + 1;

        (0..=n_points)
            .map(|point_index| {
                let distance_to_point = point_index as f32 * stride_f32;
                let fraction = distance_to_point / line_length_meters;
                let point = linestring
                    .point_at_ratio_from_start(&Haversine, fraction)
                    .ok_or_else(|| {
                        format!(
                            "unable to interpolate {}m/{}% into linestring with distance {}: {}",
                            distance_to_point,
                            (fraction * 10000.0).trunc() / 100.0,
                            line_length_meters,
                            linestring.to_wkt()
                        )
                    })?;
                Ok(point)
            })
            .collect::<Result<Vec<_>, String>>()
    }
}

pub fn accumulate_global_opps(
    opps: &[(usize, Vec<f64>)],
    colnames: &[String],
) -> Result<HashMap<String, f64>, PluginError> {
    let mut result: HashMap<String, f64> = HashMap::new();
    for (_, row) in opps.iter() {
        for (idx, value) in row.iter().enumerate() {
            let colname = colnames.get(idx).ok_or_else(|| {
                PluginError::InternalError(
                    "opportunity count row and activity types list do not match".to_string(),
                )
            })?;
            if let Some(val) = result.get_mut(colname) {
                *val += value;
            } else {
                result.insert(colname.to_string(), *value);
            }
        }
    }
    Ok(result)
}

/// steps through each bin's output section for mutable updates
pub fn iterate_bins<'a>(
    output: &'a mut serde_json::Value,
) -> Result<Box<dyn Iterator<Item = (&'a String, &'a mut serde_json::Value)> + 'a>, PluginError> {
    let bins = output.get_mut(bambam_field::TIME_BINS).ok_or_else(|| {
        PluginError::UnexpectedQueryStructure(format!(
            "after running json structure plugin, cannot find key {}",
            bambam_field::TIME_BINS
        ))
    })?;
    let bins_map = bins.as_object_mut().ok_or_else(|| {
        PluginError::UnexpectedQueryStructure(format!(
            "after running json structure plugin, field {} was not a key/value map",
            bambam_field::TIME_BINS
        ))
    })?;
    Ok(Box::new(bins_map.iter_mut()))
}
