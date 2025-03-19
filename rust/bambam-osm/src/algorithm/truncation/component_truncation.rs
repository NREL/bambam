use crate::{
    algorithm::connected_components,
    model::osm::{graph::OsmGraph, OsmError},
};
use itertools::Itertools;
use kdam::tqdm;
use std::collections::HashSet;

use super::component_filter::ComponentFilter;

/// mutates these graph assets in place so that they now represent only the largest
/// weakly-connected graph component.
pub fn filter_components(graph: &mut OsmGraph, filter: &ComponentFilter) -> Result<(), OsmError> {
    // sorted for deterministic iteration
    let all_nodes = graph.connected_node_iterator(true).cloned().collect_vec();
    let components: Vec<Vec<crate::model::osm::graph::OsmNodeId>> =
        connected_components::weakly_connected_components(graph, &all_nodes)?;
    let filtered_components = filter.assign_components(&components);
    log::info!(
        "retaining {} graph components after filtering",
        filtered_components.len()
    );

    let keep_list = filtered_components
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    let iter = tqdm!(
        all_nodes.iter(),
        desc = format!("apply {} component filter", filter),
        total = all_nodes.len()
    );
    for node_id in iter {
        if !keep_list.contains(node_id) {
            graph.disconnect_node(node_id, false)?;
        }
    }
    eprintln!();
    log::info!(
        "after filtering components, graph has {} nodes and {} segments",
        graph.n_connected_nodes(),
        graph.n_connected_ways()
    );

    Ok(())
}
