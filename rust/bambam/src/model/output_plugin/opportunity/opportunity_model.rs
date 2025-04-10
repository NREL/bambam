use std::collections::HashMap;

use super::{
    opportunity_spatial_row::OpportunitySpatialRow,
    opportunity_table_orientation::OpportunityTableOrientation,
};
use crate::model::output_plugin::mep_output_ops::DestinationsIter;
use geo::Convert;
use itertools::Itertools;
use routee_compass::plugin::output::OutputPluginError;
use routee_compass_core::{
    algorithm::search::{SearchInstance, SearchTreeBranch},
    model::network::VertexId,
};
use rstar::{RTree, RTreeObject};

/// represents activities which can become opportunities if they
/// are reached by some travel mode.
pub enum OpportunityModel {
    /// user provides a dataset with opportunity counts for each id of either
    /// vertices (source, destination) or edges in the network. assignment of
    /// opportunity counts is done by a simple lookup function.
    Tabular {
        activity_types: Vec<String>,
        activity_counts: Vec<Vec<f64>>,
        table_orientation: OpportunityTableOrientation,
    },
    /// user provides a spatial dataset of opportunities. lookup will use a
    /// spatial index to find
    /// - intersecting polygons
    /// - nearest points with some distance tolerance
    /// it becomes the responsibility of the downstream code to de-duplicate results
    /// by making sure to only include one row with a given index value (slot 1 of the
    /// attach_opportunities function result).
    Spatial {
        activity_types: Vec<String>,
        rtree: RTree<OpportunitySpatialRow>,
        activity_counts: Vec<Vec<f64>>,
        polygonal: bool,
    },
    /// Combines multiple opportunity models
    Combined { models: Vec<Box<OpportunityModel>> },
}

impl OpportunityModel {
    /// get the list of activity type names for this model.
    pub fn activity_types(&self) -> Vec<String> {
        match self {
            OpportunityModel::Tabular {
                activity_types,
                activity_counts: _,
                table_orientation: _,
            } => activity_types.to_vec(),
            OpportunityModel::Spatial {
                activity_types,
                rtree: _,
                activity_counts: _,
                polygonal: _,
            } => activity_types.to_vec(),
            OpportunityModel::Combined { models } => {
                models.iter().flat_map(|m| m.activity_types()).collect_vec()
            }
        }
    }

    pub fn vector_length(&self) -> usize {
        self.activity_types().len()
    }

    /// collect all opportunities that are reachable by some collection of destinations.
    ///
    /// # Arguments
    ///
    /// * `destinations` - an iterator over the destinations found during the search
    /// * `si` - the RouteE Compass [`SearchInstance`] for the associated search query
    ///
    /// # Returns
    ///
    /// A vector of (destination id, opportunity counts by category) for each destination id.
    /// The opportunity count vectors are ordered to match this [`OpportunityModel`]'s
    /// activity_types vector.
    pub fn batch_collect_opportunities(
        &self,
        destinations: DestinationsIter<'_>,
        si: &SearchInstance,
    ) -> Result<Vec<(usize, Vec<f64>)>, OutputPluginError> {
        let opps = collect_opps(self, destinations, si)?;
        let unique_opps = opps.into_iter().unique_by(|(id, _)| *id).collect_vec();
        Ok(unique_opps)
    }

    /// attaches opportunity counts for a single vertex.
    ///
    /// # Arguments
    /// * `destination_vertex_id` - the destination that was reached
    /// * `search_tree_branch` - the branch in the search tree that reached this destination.
    /// * `si` - the RouteE Compass [`SearchInstance`] for the associated search query
    ///
    /// # Returns
    ///
    /// an opportunity vector id along with a vector of opportunity counts.
    fn attach_opportunities(
        &self,
        destination_vertex_id: &VertexId,
        search_tree_branch: &SearchTreeBranch,
        si: &SearchInstance,
    ) -> Result<Vec<(usize, Vec<f64>)>, OutputPluginError> {
        match self {
            OpportunityModel::Tabular {
                activity_types: _,
                activity_counts,
                table_orientation,
            } => {
                use OpportunityTableOrientation as O;
                let index = match table_orientation {
                    O::OriginVertexOriented => destination_vertex_id.0,
                    O::DestinationVertexOriented => search_tree_branch.terminal_vertex.0,
                    O::EdgeOriented => search_tree_branch.edge_traversal.edge_id.0,
                };
                let result = activity_counts
                    .get(index)
                    .map(|opps| (index, opps.to_owned()))
                    .ok_or_else(|| {
                        let orientation_string = serde_json::to_string(table_orientation)
                            .unwrap_or(String::from(""))
                            .replace('\"', "");
                        OutputPluginError::OutputPluginFailed(format!(
                            "activity table lookup failed - {} index {} not found",
                            orientation_string, index
                        ))
                    })?;
                Ok(vec![result])
            }
            OpportunityModel::Spatial {
                activity_types,
                rtree,
                activity_counts,
                polygonal,
            } => {
                let graph = si.graph.clone();
                let vertex = graph
                    .get_vertex(destination_vertex_id)
                    .map_err(|e| OutputPluginError::OutputPluginFailed(e.to_string()))?;
                let point: geo::Point<f64> = geo::Point(vertex.coordinate.0).convert();
                if *polygonal {
                    let first_match = rtree
                        .locate_in_envelope_intersecting(&point.envelope())
                        .next();
                    match first_match {
                        None => Ok(vec![(
                            destination_vertex_id.0,
                            vec![0.0; activity_types.len()],
                        )]),
                        Some(nearest) => match activity_counts.get(nearest.index) {
                            Some(counts) => Ok(vec![(destination_vertex_id.0, counts.to_vec())]),
                            None => Err(OutputPluginError::OutputPluginFailed(format!(
                                "expected activity count index {} not found",
                                nearest.index
                            ))),
                        },
                    }
                } else {
                    match rtree.nearest_neighbor(&point) {
                        None => Ok(vec![(
                            destination_vertex_id.0,
                            vec![0.0; activity_types.len()],
                        )]),
                        Some(nearest) => match activity_counts.get(nearest.index) {
                            Some(counts) => Ok(vec![(destination_vertex_id.0, counts.to_vec())]),
                            None => Err(OutputPluginError::OutputPluginFailed(format!(
                                "expected activity count index {} not found",
                                nearest.index
                            ))),
                        },
                    }
                }
            }
            OpportunityModel::Combined { models } => {
                let mut collection: HashMap<usize, Vec<f64>> = HashMap::new();
                let mut padding_length: usize = 0;
                for model in models.iter() {
                    let vector_length = model.vector_length();
                    let matches = model
                        .attach_opportunities(destination_vertex_id, search_tree_branch, si)?
                        .into_iter()
                        .collect::<HashMap<_, _>>();

                    let update_indices = collection
                        .keys()
                        .cloned()
                        .chain(matches.keys().cloned())
                        .collect_vec();
                    for idx in update_indices.into_iter() {
                        let vector_extension = match matches.get(&idx) {
                            Some(match_vector) => match_vector.clone(),
                            None => vec![0.0; vector_length],
                        };

                        match collection.get_mut(&idx) {
                            Some(mut existing) => existing.extend(vector_extension),
                            None => {
                                let mut padded = vec![0.0; padding_length];
                                padded.extend(vector_extension);
                                collection.insert(idx, padded);
                            }
                        }
                    }
                    for (idx, counts) in matches.into_iter() {
                        // add these counts to our collection, left-padding the vectors as needed
                        collection
                            .entry(idx)
                            .and_modify(|c| c.extend(&counts))
                            .or_insert({
                                let mut new_counts = vec![0.0; padding_length];
                                new_counts.extend(&counts);
                                new_counts
                            });
                    }
                    padding_length += model.vector_length();
                }
                // ensure we are right-padded to the correct length as well
                let result = collection
                    .into_iter()
                    .map(|(k, mut v)| {
                        v.resize(padding_length, 0.0);
                        (k, v)
                    })
                    .collect_vec();
                Ok(result)
            }
        }
    }
}

/// helper function for collecting opportunities for some model/destinations/search instance combination.
fn collect_opps(
    model: &OpportunityModel,
    destinations: DestinationsIter<'_>,
    si: &SearchInstance,
) -> Result<Vec<(usize, Vec<f64>)>, OutputPluginError> {
    let result = destinations
        .map(|destinations_result| match destinations_result {
            Ok((src, branch)) => model.attach_opportunities(&src, branch, si),
            Err(e) => {
                let msg = format!("failure collecting destinations: {}", e);
                Err(OutputPluginError::OutputPluginFailed(msg))
            }
        })
        .collect::<Result<Vec<Vec<(usize, Vec<f64>)>>, OutputPluginError>>()?
        .into_iter()
        .flatten()
        .collect_vec();
    Ok(result)
}
