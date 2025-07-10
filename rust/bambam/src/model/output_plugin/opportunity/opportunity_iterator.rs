use crate::model::output_plugin::{
    bambam_field as field,
    isochrone::{
        isochrone_output_format::{self, IsochroneOutputFormat},
        time_bin::TimeBin,
    },
    opportunity::{
        DestinationOpportunity, OpportunityFormat, OpportunityOrientation, OpportunityRecord,
    },
};
use geo::{orient, Geometry};
use itertools::Itertools;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::{
    algorithm::search::SearchInstance,
    model::{
        map::MapModel,
        network::{EdgeId, Graph, VertexId},
    },
};
use serde_json::{json, Value};
use std::sync::Arc;

/// iterates over serialized opportunities coming from an output JSON row
pub type OpportunityIterator<'a> =
    Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>> + 'a>;

pub fn new_disaggregated<'a>(
    input: &'a Value,
    activity_types: &'a [String],
    si: &'a SearchInstance,
) -> Result<OpportunityIterator<'a>, OutputPluginError> {
    let source = disaggregated_row_iterator(input, activity_types, si);
    Ok(source)
}

pub fn new_aggregated<'a>(
    input: &'a Value,
    activity_types: &'a [String],
) -> Result<OpportunityIterator<'a>, OutputPluginError> {
    let isochrone_format = field::get::isochrone_format(input)?;
    let bin_iter = field::time_bins_iter(input).map_err(OutputPluginError::OutputPluginFailed)?;

    let source: Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>>> =
        Box::new(bin_iter.flat_map(move |bin_result| match bin_result {
            Ok((time_bin, bin_value)) => {
                aggregated_row_iterator(time_bin, bin_value, activity_types, &isochrone_format)
            }
            Err(e) => Box::new(std::iter::once(Err(OutputPluginError::OutputPluginFailed(
                format!("failure reading bins from output: {}", e),
            )))),
        }));

    Ok(source)
}

fn deserialize_geometry(
    bin_value: &Value,
    isochrone_format: &IsochroneOutputFormat,
) -> Result<Geometry<f32>, OutputPluginError> {
    let geometry_json = bin_value.get(field::ISOCHRONE).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(String::from("missing isochrone for time bin"))
    })?;
    let geometry = isochrone_format.deserialize_geometry(geometry_json)?;
    Ok(geometry)
}

fn disaggregated_row_iterator<'a>(
    value: &'a Value,
    activity_types: &'a [String],
    si: &'a SearchInstance,
) -> Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>> + 'a> {
    let opportunities_result = value.get(field::OPPORTUNITIES).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(String::from("missing opportunities for row"))
    });
    let opportunities_json = match opportunities_result {
        Ok(o) => o,
        Err(e) => return Box::new(std::iter::once(Err(e))),
    };
    let opportunities_opt = opportunities_json.as_object();
    let opportunities_obj = match opportunities_opt {
        Some(o) => o,
        None => {
            return Box::new(std::iter::once(Err(OutputPluginError::InternalError(
                format!("disaggregate opportunities should be stored in a map"),
            ))))
        }
    };

    let result = opportunities_obj.iter().flat_map(|(k, v)| {
        // each opportunity could have come from a different opportunity source, so we get the orientation here.
        let opportunity_orientation = match field::get::opportunity_orientation(&v) {
            Ok(o) => o,
            Err(e) => return Box::new(std::iter::once(Err(e))) as Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>>>,
        };
        // the identifier is a serialized EdgeId or VertexId
        let id_result: Result<usize, _> = k.parse().map_err(|e| OutputPluginError::InternalError(format!("disaggregate opportunity should be stored alongside numeric graph element identifier, an integer, but found {}", k)));
        let id = match id_result {
            Ok(i) => i,
            Err(e) => return Box::new(std::iter::once(Err(e))) as Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>>>,
        };

        // the associated geometry is a Vertex coordinate or an Edge LineString, which we can grab from the map or graph
        let geometry_result = geometry_from_map(id, &opportunity_orientation, si.graph.clone(), si.map_model.clone());
        let geometry = match geometry_result {
            Ok(g) => g,
            Err(e) => return Box::new(std::iter::once(Err(e))) as Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>>>,
        };

        // pull the values (counts, state) from the JSON
        let row_result: Result<DestinationOpportunity, _> = serde_json::from_value(v.clone()).map_err(|e| OutputPluginError::OutputPluginFailed(format!("disaggregate opportunity '{}' has unexpected shape: {}", id, e)));
        let row = match row_result {
            Ok(r) => r,
            Err(e) => {
                return Box::new(std::iter::once(Err(e)))
            }
        };

        // deserialize the opportunity counts stored on this row
        let inner_result = activity_types.iter().zip(row.counts).map(move |(act, count)| Ok(OpportunityRecord::Disaggregate {
                id: k.clone(),
                opportunity_orientation,
                activity_type: act.clone(),
                geometry: geometry.clone(),
                state: row.state.clone(),
            }));
        Box::new(inner_result)
    });
    Box::new(result)
}

fn aggregated_row_iterator<'a>(
    time_bin: TimeBin,
    value: &'a Value,
    activity_types: &'a [String],
    isochrone_format: &IsochroneOutputFormat,
) -> Box<dyn Iterator<Item = Result<OpportunityRecord, OutputPluginError>> + 'a> {
    let geometry_json_result = value.get(field::ISOCHRONE).ok_or_else(|| {
        OutputPluginError::OutputPluginFailed(format!(
            "missing isochrone for time bin {}",
            time_bin.max_time
        ))
    });

    let geometry_json = match geometry_json_result {
        Ok(gj) => gj,
        Err(e) => return Box::new(std::iter::once(Err(e))),
    };

    let geometry_result = isochrone_format.deserialize_geometry(geometry_json);
    let geometry = match geometry_result {
        Ok(g) => g,
        Err(e) => return Box::new(std::iter::once(Err(e))),
    };

    let opportunities_result = value.get(field::OPPORTUNITIES).ok_or_else(|| {
        let keys = match value.as_object() {
            Some(o) => o.keys().map(|k| k.to_string()).join(", "),
            None => String::from("internal error! response is not a JSON Object"),
        };

        OutputPluginError::OutputPluginFailed(format!(
            "missing opportunities for time bin {}, found keys: [{}]",
            time_bin.key(),
            keys
        ))
    });
    let opportunities = match opportunities_result {
        Ok(o) => o,
        Err(e) => return Box::new(std::iter::once(Err(e))),
    };

    let inner = deserialize_row(opportunities, activity_types).map(move |deserialize_result| {
        deserialize_result.map(|(act, count)| OpportunityRecord::Aggregate {
            activity_type: act.clone(),
            geometry: geometry.clone(),
            time_bin: time_bin.clone(),
            count,
        })
    });
    Box::new(inner)
}

fn deserialize_row<'a>(
    input: &'a serde_json::Value,
    activity_types: &'a [String],
) -> Box<dyn Iterator<Item = Result<(&'a String, f64), OutputPluginError>> + 'a> {
    let iter = activity_types.iter().map(|act| {
        let count_json = input.get(act).ok_or_else(|| {
            OutputPluginError::OutputPluginFailed(format!(
                "activity type '{}' missing from aggregate opportunity data",
                act
            ))
        })?;
        let count = count_json.as_f64().ok_or_else(|| {
            OutputPluginError::OutputPluginFailed(format!(
                "activity count value for '{}' is not a valid number: '{}'",
                act, count_json
            ))
        })?;
        Ok((act, count))
    });
    Box::new(iter)
}

fn geometry_from_map(
    id: usize,
    orientation: &OpportunityOrientation,
    graph: Arc<Graph>,
    map_model: Arc<MapModel>,
) -> Result<Geometry<f32>, OutputPluginError> {
    use OpportunityOrientation as O;
    match orientation {
        O::OriginVertexOriented | O::DestinationVertexOriented => {
            let vertex = graph.get_vertex(&VertexId(id)).map_err(|e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "vertex-oriented disaggregate opportunity has unknown vertex_id '{}'",
                    id
                ))
            })?;
            Ok(geo::Geometry::Point(geo::Point::new(
                vertex.coordinate.x,
                vertex.coordinate.y,
            )))
        }
        O::EdgeOriented => {
            let linestring = map_model.get(&EdgeId(id)).map_err(|e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "edge-oriented disaggregate opportunity has unknown edge_id '{}'",
                    id
                ))
            })?;
            Ok(geo::Geometry::LineString(linestring.clone()))
        }
    }
}
