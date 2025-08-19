use crate::model::osm::{
    graph::{AdjacencyDirection, OsmGraph, OsmNodeId},
    OsmError,
};
use itertools::Itertools;
use std::collections::{HashSet, LinkedList};

/// finds the set of indices that are part of the same geometry cluster
/// using a breadth-first search over an undirected graph of geometry
/// intersection relations.
///
/// # Arguments
///
/// * `src` - origin of tree
/// * `graph` - graph to search
/// * `valid_set` - set of valid nodes to visit, or None if all are acceptable.
///
/// # Returns
///
/// The set of node ids connected by the minimum spanning tree within the valid_set.
pub fn bfs_undirected(
    src: OsmNodeId,
    graph: &OsmGraph,
    valid_set: Option<&HashSet<OsmNodeId>>,
) -> Result<HashSet<OsmNodeId>, OsmError> {
    // initial search state. if a NodeOsmid has been visited, it is appended
    // to the visited set.
    // breadth-first search is modeled here with a linked list FIFO queue.
    let mut visited: HashSet<OsmNodeId> = HashSet::new();
    let mut frontier: LinkedList<OsmNodeId> = LinkedList::new();
    frontier.push_back(src);

    while let Some(next_id) = frontier.pop_front() {
        // add to the search tree
        visited.insert(next_id);

        // expand the frontier
        let next_out = graph.get_out_neighbors(&next_id).unwrap_or_default();
        let next_in = graph.get_in_neighbors(&next_id).unwrap_or_default();

        // neighbors are only reviewed here that are "valid" (all fall within the spatial cluster).
        // they are sorted for algorithmic determinism (frontier insertion order).
        let valid_neighbors = next_in
            .union(&next_out)
            .filter(|n| match &valid_set {
                Some(valid) => valid.contains(*n),
                None => true,
            })
            .sorted();
        for neighbor in valid_neighbors {
            if !visited.contains(neighbor) {
                // frontier.push(Reverse((next_depth + 1, *neighbor))); // min heap
                frontier.push_back(*neighbor); // min heap
            }
        }
    }

    Ok(visited)
}
