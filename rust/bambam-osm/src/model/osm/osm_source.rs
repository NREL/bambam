use super::{graph::osm_element_filter::ElementFilter, OsmError};
use crate::{
    algorithm::{
        consolidation, simplification,
        truncation::{self, ComponentFilter},
    },
    model::osm::{
        graph::{OsmGraph, OsmGraphVectorized, OsmNodeId},
        import_ops,
    },
};
use geo::{Convert, Geometry, MultiPolygon};
use geo_buffer;
use itertools::Itertools;
use routee_compass_core::model::unit::{Distance, DistanceUnit};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wkt::{ToWkt, TryFromWkt};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OsmSource {
    Pbf {
        pbf_filepath: String,
        network_filter: Option<ElementFilter>,
        extent_filter_filepath: Option<String>,
        component_filter: Option<ComponentFilter>,
        truncate_by_edge: bool,
        ignore_errors: bool,
        simplify: bool,
        consolidate: bool,
        consolidation_threshold: (Distance, DistanceUnit),
        parallelize: bool,
    },
}

impl OsmSource {
    pub fn import(&self) -> Result<OsmGraphVectorized, OsmError> {
        match self {
            OsmSource::Pbf {
                pbf_filepath,
                network_filter,
                extent_filter_filepath,
                component_filter,
                truncate_by_edge,
                ignore_errors,
                simplify,
                consolidate,
                consolidation_threshold,
                parallelize,
            } => {
                let net_ftr = network_filter.clone().unwrap_or_default();
                let extent_opt = read_extent_wkt(extent_filter_filepath)?;
                let cc_ftr = component_filter.clone().unwrap_or_default();

                // # download the network data from OSM within buffered polygon
                // # create buffered graph from the downloaded data
                eprintln!();
                log::info!("  (((1))) reading PBF source");
                let (nodes, ways) = import_ops::read_pbf(pbf_filepath, net_ftr, &extent_opt)?;
                let mut graph = OsmGraph::new(nodes, ways)?;

                // rjf: this is handled above in import_ops::read_pbf for performance reasons
                // # truncate buffered graph to the buffered polygon and retain_all for
                // # now. needed because overpass returns entire ways that also include
                // # nodes outside the poly if the way (that is, a way with a single OSM
                // # ID) has a node inside the poly at some point.
                // G_buff = truncate.truncate_graph_polygon(G_buff, poly_buff, truncate_by_edge=truncate_by_edge)
                // if let Some(extent) = &extent_opt {
                //     let extent_buffered = buffer_extent(extent, Self::BUFFER_500M_IN_DEGREES)?;
                //     truncation::truncate_graph_polygon(&mut graph, extent, *truncate_by_edge)?;
                // }

                // # keep only the largest weakly connected component if retain_all is False
                // if not retain_all:
                // G_buff = truncate.largest_component(G_buff, strongly=False)
                eprintln!();
                log::info!("  (((2))) truncating graph via connected components filtering");
                truncation::filter_components(&mut graph, &cc_ftr)?;

                let mut apply_second_component_filter = false;
                if *simplify {
                    eprintln!();
                    log::info!("  (((3))) simplifying graph");
                    simplification::simplify_graph(&mut graph, *parallelize)?;
                    apply_second_component_filter = true;
                } else {
                    eprintln!();
                    log::info!("  (((3))) simplifying graph (skipped)");
                }

                // # truncate graph by original polygon to return graph within polygon
                // # caller wants. don't *simplify again: this allows us to retain
                // # intersections along the street that may now only connect 2 street
                // # segments in the network, but in reality also connect to an
                // # intersection just outside the polygon
                // G = truncate.truncate_graph_polygon(G_buff, polygon, truncate_by_edge=truncate_by_edge)
                if let Some(extent) = &extent_opt {
                    eprintln!();
                    log::info!("  (((4))) truncating graph via extent filtering");
                    truncation::truncate_graph_polygon(
                        &mut graph,
                        extent,
                        *truncate_by_edge,
                        *ignore_errors,
                    )?;
                    apply_second_component_filter = true;
                } else {
                    eprintln!();
                    log::info!("  (((4))) truncating graph via extent filtering (skipped)");
                }

                // # keep only the largest weakly connected component if retain_all is False
                // # we're doing this again in case the last truncate disconnected anything
                // # on the periphery
                // if not retain_all:
                // G = truncate.largest_component(G, strongly=False)
                if apply_second_component_filter {
                    eprintln!();
                    log::info!("  (((5))) truncating graph via connected components filtering");
                    truncation::filter_components(&mut graph, &cc_ftr)?;
                } else {
                    eprintln!();
                    log::info!(
                        "  (((5))) truncating graph via connected components filtering (skipped)"
                    );
                }

                // if requested, consolidate nodes in the graph
                if *consolidate {
                    eprintln!();
                    log::info!("  (((6))) consolidating graph nodes");
                    consolidation::consolidate_graph(&mut graph, *consolidation_threshold, false)?;
                } else {
                    eprintln!();
                    log::info!("  (((6))) consolidating graph nodes (skipped)");
                }

                // finalize the graph via vectorization
                let result = OsmGraphVectorized::new(graph)?;

                log::info!(
                    "loaded PBF-sourced Compass graph with {} nodes, {} ways",
                    result.nodes.len(),
                    result.ways.len()
                );
                Ok(result)
            }
        }
    }
}

/// helper function that attempts to read an optional WKT from a file if provided.
fn read_extent_wkt(
    extent_filter_filepath: &Option<String>,
) -> Result<Option<Geometry<f32>>, OsmError> {
    let extent_filter_result = extent_filter_filepath.as_ref().map(|eff| {
        let wkt_str = std::fs::read_to_string(eff).map_err(|e| {
            OsmError::ConfigurationError(format!("unable to read file {}: {}", eff, e))
        })?;
        let geom: Result<Geometry<f32>, OsmError> =
            Geometry::try_from_wkt_str(&wkt_str).map_err(|e| {
                OsmError::ConfigurationError(format!("unable to read WKT in {}: {}", eff, e))
            });
        geom
    });
    match extent_filter_result {
        Some(Ok(f)) => Ok(Some(f)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}
