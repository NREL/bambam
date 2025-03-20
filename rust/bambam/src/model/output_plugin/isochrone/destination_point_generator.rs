use geo::{Densify, LineString, MultiPoint, Point};
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::{
    algorithm::search::SearchTreeBranch,
    model::{
        map::MapModel,
        network::{EdgeId, VertexId},
        unit::{AsF64, Convert, Distance, DistanceUnit},
    },
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum DestinationPointGenerator {
    DestinationPoint,
    LinestringCoordinates,
    LinestringStride {
        stride: Distance,
        distance_unit: DistanceUnit,
    },
    BufferedLinestring {
        buffer_radius: Distance,
        buffer_stride: Distance,
        distance_unit: DistanceUnit,
    },
    BufferedDestinationPoint {
        buffer_radius: Distance,
        buffer_stride: Distance,
        distance_unit: DistanceUnit,
    },
}

impl DestinationPointGenerator {
    pub fn generate_destination_points(
        &self,
        destinations: &[(VertexId, &SearchTreeBranch)],
        map_model: Arc<MapModel>,
    ) -> Result<MultiPoint<f32>, OutputPluginError> {
        let mut result: Vec<Point<f32>> = Vec::new();
        for (_v_id, branch) in destinations.iter() {
            let edge_id = branch.edge_traversal.edge_id;
            let linestring = map_model.get(&edge_id).map_err(|e| {
                OutputPluginError::OutputPluginFailed(format!(
                    "failure generating destination points: {}",
                    e
                ))
            })?;
            let points = self.linestring_to_points(edge_id, linestring)?;
            result.extend(points);
        }

        let mp = MultiPoint::new(result);
        Ok(mp)
    }

    pub fn linestring_to_points(
        &self,
        edge_id: EdgeId,
        linestring: &LineString<f32>,
    ) -> Result<Vec<Point<f32>>, OutputPluginError> {
        match self {
            DestinationPointGenerator::DestinationPoint => {
                let last_point = linestring.points().last().ok_or_else(|| {
                    OutputPluginError::OutputPluginFailed(format!(
                        "geometry for edge_id {} has no points",
                        edge_id,
                    ))
                })?;
                Ok(vec![last_point])
            }
            DestinationPointGenerator::LinestringCoordinates => Ok(linestring.points().collect()),
            DestinationPointGenerator::LinestringStride {
                stride,
                distance_unit,
            } => {
                let mut meters = Cow::Borrowed(stride);
                distance_unit
                    .convert(&mut meters, &DistanceUnit::Meters)
                    .map_err(|e| {
                        OutputPluginError::OutputPluginFailed(format!(
                            "failure converting stride {} to meters: {}",
                            stride, e
                        ))
                    })?;
                let dense_linestring = linestring.densify(meters.as_f64() as f32);
                Ok(dense_linestring.into_points())
            }
            DestinationPointGenerator::BufferedLinestring {
                buffer_radius: _,
                buffer_stride: _,
                distance_unit: _,
            } => {
                todo!("geo rust does not currently support geometry buffering")
            }
            DestinationPointGenerator::BufferedDestinationPoint {
                buffer_radius: _,
                buffer_stride: _,
                distance_unit: _,
            } => todo!("geo rust does not currently support geometry buffering"),
        }
    }
}
