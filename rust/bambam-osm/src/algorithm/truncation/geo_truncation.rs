use crate::model::osm::{graph::OsmGraph, OsmError};
use geo::{Contains, Geometry};
use itertools::Itertools;
use rayon::prelude::*;
use std::sync::Arc;

/// removes nodes that are outside of the provided extent
pub fn truncate_graph_polygon(
    graph: &mut OsmGraph,
    extent: &Geometry<f32>,
    truncate_by_edge: bool,
    ignore_errors: bool,
) -> Result<(), OsmError> {
    // msg = "Identifying all nodes that lie outside the polygon..."
    // utils.log(msg, level=lg.INFO)
    log::info!("identifying all nodes that lie outside the polygon");

    // # first identify all nodes whose point geometries lie within the polygon
    // gs_nodes = convert.graph_to_gdfs(G, edges=False)["geometry"]
    // to_keep = utils_geo._intersect_index_quadrats(gs_nodes, polygon)
    match extent {
        Geometry::Polygon(_) => {}
        Geometry::MultiPolygon(_) => {}
        _ => {
            return Err(OsmError::ConfigurationError(String::from(
                "import extent must be a POLYGON or MULTIPOLYGON",
            )))
        }
    }

    let n_removed = if truncate_by_edge {
        truncate_graph_by_edge(graph, extent, !ignore_errors)?
    } else {
        truncate_graph_by_node(graph, extent, !ignore_errors)?
    };

    let done_msg = if truncate_by_edge {
        format!(
            "removed {n_removed} nodes not connected by edges to nodes within the provided extent"
        )
    } else {
        format!("removed {n_removed} nodes found outside the provided extent")
    };
    log::info!("{done_msg}");
    // msg = "Truncated graph by polygon"
    // utils.log(msg, level=lg.INFO)
    // return G

    Ok(())
}

/// removes nodes which are not contained within the extent, but also keep nodes that are connected
/// via segments to nodes which are within the extent.
fn truncate_graph_by_edge(
    graph: &mut OsmGraph,
    extent: &Geometry<f32>,
    fail_if_missing: bool,
) -> Result<usize, OsmError> {
    let shared_extent = Arc::new(extent);
    let remove_segments = {
        let shared_graph = Arc::new(&graph);
        shared_graph
            .connected_multiedge_way_triplet_iterator(false)
            .par_bridge()
            .filter_map(|result| {
                let triplets = match result {
                    Err(_) => return None,
                    Ok(None) => return None,
                    Ok(Some(triplets)) => triplets,
                };
                triplets
                    .iter()
                    .find(|(src_node, _, dst_node)| {
                        let inner_graph = shared_graph.clone();
                        let inner_extent = extent.clone();

                        let src_in_extent = inner_extent.contains(&src_node.get_point());
                        let dst_in_extent = inner_extent.contains(&dst_node.get_point());
                        !(src_in_extent || dst_in_extent)
                    })
                    .map(|(src, _, dst)| (src.osmid, dst.osmid))
            })
            .collect::<Vec<_>>()
    };
    let n_removed = remove_segments.len();
    for (src, dst) in remove_segments.into_iter() {
        // perhaps fail should be "true" but i think there can be duplicates (?), so we
        // would need to be able to account for that.
        graph.remove_way(&src, &dst, fail_if_missing)?;
    }

    Ok(n_removed)
}

/// removes nodes that are not contained by the extent.
fn truncate_graph_by_node(
    graph: &mut OsmGraph,
    extent: &Geometry<f32>,
    fail_if_missing: bool,
) -> Result<usize, OsmError> {
    let shared_extent = Arc::new(extent);
    let remove_nodes = {
        let shared_graph = Arc::new(&graph);
        let outer = shared_graph.clone();
        outer
            .connected_node_data_iterator(false)
            .par_bridge()
            .map(|result| {
                let node = result?;
                let inner_graph = shared_graph.clone();
                let inner_extent = extent.clone();
                let point = node.get_point();
                if inner_extent.contains(&point) {
                    Ok(None) // we are returning points TO REMOVE here
                } else {
                    Ok(Some(node.osmid))
                }
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect_vec()
    };
    let n_removed = remove_nodes.len();

    for node_id in remove_nodes.into_iter() {
        graph.disconnect_node(&node_id, fail_if_missing)?;
    }

    Ok(n_removed)
}
