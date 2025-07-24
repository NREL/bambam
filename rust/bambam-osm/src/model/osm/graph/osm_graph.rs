use super::{
    osm_node_data::OsmNodeData, osm_segment::OsmSegment, osm_way_data::OsmWayData,
    AdjacencyDirection as Dir, AdjacencyList, AdjacencyList3, AdjacencyListDeprecated, OsmNodeId,
    OsmNodes, OsmWayId, OsmWays, OsmWaysByOd,
};
use crate::{algorithm::simplification::SimplifiedPath, model::osm::OsmError};
use geo::LineString;
use itertools::Itertools;
use kdam::tqdm;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use wkt::ToWkt;

pub type TripletRow<'a> = Result<Vec<(&'a OsmNodeData, &'a OsmWayData, &'a OsmNodeData)>, OsmError>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct OsmGraph {
    /// the collection of OSM nodes associated via their OSMID
    nodes: OsmNodes,
    /// ways are stored wrt their src/dst node pairs
    ways: OsmWaysByOd,
    /// forward and reverse adjacency list
    adj: AdjacencyList3,
}

/// internal enumeration used to disambiguate graph update methods associated with
/// the adjacency list.
enum WayOverwritePolicy {
    /// simply append a new way onto the multiedges on this relation
    Append,
    /// way update: fail if there is no previously-existing way on this relation/index
    UpdateAtIndex { index: usize },
    /// overwrite the set of multiedges on this relation
    Replace,
    // /// way replacement: if a previously-existing way exists, use the OSM Highway
    // /// tag as a tie-breaker to determine which way to keep.
    // CompareExistingHighwayTag,
}

impl OsmGraph {
    /// creates a new graph to model the relationship between the provided
    /// nodes and ways.
    pub fn new(nodes: OsmNodes, ways: OsmWays) -> Result<OsmGraph, OsmError> {
        let mut ways_by_od: OsmWaysByOd = HashMap::new();
        let mut adj: AdjacencyList3 = HashMap::new();
        for way in ways.into_values() {
            let way_clone = way.clone();
            let nodes = way.nodes.clone();
            for (src, dst) in nodes.into_iter().tuple_windows() {
                // store a copy of this way between these src, dst nodes
                ways_by_od
                    .entry((src, dst))
                    .and_modify(|v| v.push(way_clone.clone()))
                    .or_insert(vec![way_clone.clone()]);

                // update the adjacency list
                match adj.get_mut(&(src, Dir::Forward)) {
                    None => {
                        let _ = adj.insert((src, Dir::Forward), HashSet::from([dst]));
                    }
                    Some(adjacencies) => {
                        let _ = adjacencies.insert(dst);
                    }
                }
                match adj.get_mut(&(dst, Dir::Reverse)) {
                    None => {
                        let _ = adj.insert((dst, Dir::Reverse), HashSet::from([src]));
                    }
                    Some(adjacencies) => {
                        let _ = adjacencies.insert(src);
                    }
                }
            }
        }
        Ok(OsmGraph {
            nodes,
            ways: ways_by_od,
            adj,
        })
    }

    /// returns the number of connected nodes found in the adjacency list.
    pub fn n_connected_nodes(&self) -> usize {
        // takes account of the fact that, when any node is connected, it has
        // exactly two entries (fwd + rev) in the adjacency list
        self.adj.len() / 2
    }

    /// expensive-ish operation used to get the count of segments in the adjacency list.
    /// when tested on a city-sized input, this ran in 12351235.00 iterations per second,
    /// so perhaps we don't need this disclaimer, but this may change as network sizes increase.
    /// since, for each segment, there is a forward and reverse entry, we simply ignore
    /// the reverse entries in the count.
    ///
    /// since this is a multigraph, there may be more than 1 edge between some pair (u, v)
    pub fn n_connected_ways(&self) -> usize {
        self.ways
            .values()
            .map(|multiedges| multiedges.len())
            .sum::<usize>()
    }

    /// helper with error handling for getting the node data for a given node id
    pub fn get_node_data(&self, node_id: &OsmNodeId) -> Result<&OsmNodeData, OsmError> {
        self.nodes
            .get(node_id)
            .ok_or(OsmError::GraphMissingNodeId(*node_id))
    }

    // /// helper with error handling for getting the way data for a given way id
    // pub fn get_way_data(&self, way_id: &OsmWayId) -> Result<&OsmWayData, OsmError> {
    //     self.ways
    //         .get(way_id)
    //         .ok_or({ OsmError::GraphMissingWayId(*way_id) })
    // }

    /// helper with error handling to retrieve the "out-edges" as a hashmap
    /// from destination node to the segment connecting origin to destination
    pub fn get_neighbors(&self, node_id: &OsmNodeId, direction: Dir) -> Option<HashSet<OsmNodeId>> {
        self.adj.get(&(*node_id, Dir::Forward)).cloned()
    }

    /// helper with error handling to retrieve the "out-edges" as a hashmap
    /// from destination node to the segment connecting origin to destination
    /// fails if empty.
    pub fn get_out_neighbors(&self, origin: &OsmNodeId) -> Option<HashSet<OsmNodeId>> {
        self.get_neighbors(origin, Dir::Forward)
    }

    /// helper with error handling to retrieve the "in-edges" as a hashmap
    /// from origin node to the segment connecting origin to destination
    /// fails if empty
    pub fn get_in_neighbors(&self, destination: &OsmNodeId) -> Option<HashSet<OsmNodeId>> {
        self.get_neighbors(destination, Dir::Reverse)
    }

    /// helper with error handling to retrieve the [`OsmSegment`]s describing
    /// the way connecting origin and destination.
    /// fails if empty
    pub fn get_ways_from_od(
        &self,
        origin: &OsmNodeId,
        destination: &OsmNodeId,
    ) -> Result<&Vec<OsmWayData>, OsmError> {
        self.ways.get(&(*origin, *destination)).ok_or(
            OsmError::AdjacencyWithSourceMissingDestinationNodeId(
                *origin,
                Dir::Forward,
                *destination,
            ),
        )
    }

    /// reports the node degree for some node for only a particular segment direction
    pub fn node_degree_for_direction(&self, node_id: &OsmNodeId, dir: Dir) -> Option<usize> {
        self.adj.get(&(*node_id, dir)).map(|adj| adj.len())
    }

    /// reports the node degree (number of connections) for both forward and reverse segments
    pub fn node_degree(&self, node_id: &OsmNodeId) -> Option<usize> {
        match (
            self.node_degree_for_direction(node_id, Dir::Forward),
            self.node_degree_for_direction(node_id, Dir::Reverse),
        ) {
            (None, None) => None,
            (None, Some(r)) => Some(r),
            (Some(f), None) => Some(f),
            (Some(f), Some(r)) => Some(f + r),
        }
    }

    /// gives the set of neighbor ids for either direction from this node
    pub fn node_neighbors(&self, node_id: &OsmNodeId) -> Option<HashSet<OsmNodeId>> {
        match (
            self.get_neighbors(node_id, Dir::Forward),
            self.get_neighbors(node_id, Dir::Reverse),
        ) {
            (None, None) => None,
            (None, Some(r)) => Some(r.clone()),
            (Some(f), None) => Some(f.clone()),
            (Some(f), Some(r)) => Some(f.union(&r).cloned().collect::<HashSet<_>>()),
        }
    }

    // checks if a given node_id has a neighbor with another given id
    pub fn has_neighbor(
        &self,
        node_id: &OsmNodeId,
        neighbor_id: &OsmNodeId,
        direction: Option<Dir>,
    ) -> bool {
        match direction {
            Some(dir) => match self.adj.get(&(*node_id, dir)) {
                Some(adjacencies) => adjacencies.contains(neighbor_id),
                None => false,
            },
            None => {
                match (
                    self.adj.get(&(*node_id, Dir::Forward)),
                    self.adj.get(&(*node_id, Dir::Forward)),
                ) {
                    (None, None) => false,
                    (None, Some(b)) => b.contains(neighbor_id),
                    (Some(a), None) => a.contains(neighbor_id),
                    (Some(a), Some(b)) => a.contains(neighbor_id) || b.contains(neighbor_id),
                }
            }
        }
    }

    /// iterator that returns each node in the adjacency list, sorted by id.
    ///
    /// this iterator is constructed over the node adjacency list, since,
    /// at a given time, the collection self.nodes, which contains the original (raw)
    /// osm node data may contain nodes which have been completely removed from the adjacency list.
    /// this iterator returns only those nodes that are still "alive" aka connected.
    ///
    /// the iterator is first sorted in order to guarantee idempotency on repeated runs.
    pub fn connected_node_iterator<'a>(
        &'a self,
        sorted: bool,
    ) -> Box<dyn Iterator<Item = &'a OsmNodeId> + 'a + Send + Sync> {
        let iter = tqdm!(
            self.adj
                .iter()
                .flat_map(|((src, dir), adjacencies)| match dir {
                    Dir::Reverse => None,
                    Dir::Forward => Some(src),
                }),
            desc = "sort nodes for iteration",
            total = self.adj.len()
        );
        if sorted {
            let sorted_iter = iter.sorted_by_cached_key(|n| n.0);
            Box::new(sorted_iter)
        } else {
            Box::new(iter)
        }
    }

    pub fn neighbor_iterator<'a>(
        &'a self,
        node_id: &OsmNodeId,
        direction: Dir,
    ) -> Box<dyn Iterator<Item = &'a OsmNodeId> + 'a + Send + Sync> {
        match self.adj.get(&(*node_id, direction)) {
            Some(adjacencies) => Box::new(adjacencies.iter()),
            None => Box::new(std::iter::empty()),
        }
    }

    /// iterator that returns each node in the adjacency list, sorted by id.
    ///
    /// this iterator is constructed over the node adjacency list, since,
    /// at a given time, the collection self.nodes, which contains the original (raw)
    /// osm node data may contain nodes which have been completely removed from the adjacency list.
    /// this iterator returns only those nodes that are still "alive" aka connected.
    ///
    /// the iterator is first sorted in order to guarantee idempotency on repeated runs.
    pub fn connected_node_pair_iterator<'a>(
        &'a self,
        sorted: bool,
    ) -> Box<dyn Iterator<Item = (&'a OsmNodeId, &'a OsmNodeId)> + 'a + Send + Sync> {
        let iter = tqdm!(
            self.adj
                .iter()
                .flat_map(|((src, dir), adjacencies)| match dir {
                    Dir::Forward => adjacencies.iter().map(|dst| (src, dst)).collect_vec(),
                    Dir::Reverse => vec![],
                }),
            desc = "sort nodes for iteration",
            total = self.adj.len()
        );
        if sorted {
            let sorted_iter = iter.sorted();
            Box::new(sorted_iter)
        } else {
            Box::new(iter)
        }
    }

    /// iterator that returns each node in the adjacency list, sorted by way id.
    ///
    /// note that, at a given time, the collection self.nodes, which contains the original (raw)
    /// osm node data may contain nodes which have been completely removed from the adjacency list.
    /// this iterator returns only those nodes that are still "alive" aka connected.
    ///
    /// the iterator is first sorted in order to guarantee idempotency on repeated runs.
    /// each row is a result since the node lookup can theoretically fail with invalid graph data.
    pub fn connected_node_data_iterator<'a>(
        &'a self,
        sorted: bool,
    ) -> Box<dyn Iterator<Item = Result<(&'a OsmNodeData), OsmError>> + 'a + Send + Sync> {
        let iter = tqdm!(
            self.adj
                .iter()
                .flat_map(|((src, dir), adjacencies)| match dir {
                    Dir::Reverse => None,
                    Dir::Forward => match self.nodes.get(src) {
                        None => Some(Err(OsmError::InternalError(format!(
                            "node data for node '{src}' missing from graph"
                        )))),
                        Some(node_data) => Some(Ok(node_data)),
                    },
                }),
            desc = "sort node and data for iteration",
            total = self.adj.len()
        );
        if sorted {
            let sorted_iter = iter.sorted_by_cached_key(|result| match result {
                Ok(n) => n.osmid.0,
                Err(_) => i64::MIN,
            });
            Box::new(sorted_iter)
        } else {
            Box::new(iter)
        }
    }

    /// iterator that returns each triple in the adjacency list of (src node) -[segment]-> (dst node)
    ///
    /// the iterator is first sorted in order to guarantee idempotency on repeated runs.
    pub fn connected_ways_triplet_iterator<'a>(
        &'a self,
        sorted: bool,
    ) -> Box<dyn Iterator<Item = TripletRow<'a>> + 'a + Send + Sync> {
        let desc = if sorted {
            "collect sorted adjacencies for edge list"
        } else {
            "collect adjancencies for edge list"
        };
        let iter = tqdm!(
            self.adj.iter().map(|((src, dir), adjacencies)| match dir {
                Dir::Reverse => Ok(vec![]),
                Dir::Forward => {
                    let inner = adjacencies
                        .iter()
                        .map(|dst| {
                            let src_node = self.get_node_data(src)?;
                            let dst_node = self.get_node_data(dst)?;
                            let triplets = self
                                .get_ways_from_od(src, dst)?
                                .iter()
                                .map(|w| (src_node, w, dst_node))
                                .collect_vec();
                            Ok(triplets)
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .flatten()
                        .collect_vec();
                    Ok(inner)
                }
            }),
            desc = desc,
            total = self.adj.len()
        );
        if sorted {
            // sort by the first segment id. assumed here that the None branch is never reached and that failure due to
            // empty segments would be raised elsewhere.
            let sorted_iter = iter.sorted_by_cached_key(|result| match result {
                Ok(ways) => match ways.first() {
                    Some((_, way, _)) => way.osmid.0,
                    None => i64::MIN,
                },
                Err(_) => i64::MIN,
            });
            Box::new(sorted_iter)
        } else {
            Box::new(iter)
        }
    }

    pub fn out_multiedge_iterator<'a>(
        &'a self,
        origin: &'a OsmNodeId,
    ) -> Box<dyn Iterator<Item = Result<&'a Vec<OsmWayData>, OsmError>> + 'a> {
        Box::new(
            self.get_out_neighbors(origin)
                .unwrap_or_default()
                .into_iter()
                .map(|destination| self.get_ways_from_od(origin, &destination)),
        )
    }

    pub fn in_multiedge_iterator<'a>(
        &'a self,
        destination: &'a OsmNodeId,
    ) -> Box<dyn Iterator<Item = Result<&'a Vec<OsmWayData>, OsmError>> + 'a> {
        Box::new(
            self.get_in_neighbors(destination)
                .unwrap_or_default()
                .into_iter()
                .map(|origin| self.get_ways_from_od(&origin, destination)),
        )
    }

    /// add just the node data to the nodes collection.
    /// ignores the adjacency list and node count.
    pub fn add_node_only(&mut self, node: OsmNodeData) -> Result<(), OsmError> {
        let node_id = node.osmid;
        if self.nodes.insert(node_id, node).is_some() {
            return Err(OsmError::InvalidOsmData(format!(
                "attempting to insert node {node_id} already present in graph"
            )));
        }
        Ok(())
    }

    /// adds a node to the graph.
    ///
    /// # Arguments
    /// * `node` - node to add
    /// * `adjacencies` - if provided, a list of adjacencies to add to the graph
    pub fn insert_and_attach_node(
        &mut self,
        node: OsmNodeData,
        adjacencies: Option<Vec<(OsmNodeId, Vec<OsmWayData>, OsmNodeId)>>,
    ) -> Result<(), OsmError> {
        let node_id = node.osmid;

        self.add_node_only(node)?;
        self.intialize_adjacency(&node_id)?;

        if let Some(adj) = adjacencies {
            for (src, segs, dst) in adj.into_iter() {
                self.add_new_adjacency(&src, &dst, segs)?;
            }
        }
        Ok(())
    }

    /// adds/appends a directed way between two nodes
    ///
    /// # Arguments
    /// * `src` - source node
    /// * `dst` - destination node
    /// * `segments` - references to OSM Ways which combine to describe
    /// * `overwrite_policy` - define the intention when existing ways are found
    pub fn add_new_adjacency(
        &mut self,
        src: &OsmNodeId,
        dst: &OsmNodeId,
        ways: Vec<OsmWayData>,
    ) -> Result<(), OsmError> {
        add_ways_to_graph(self, src, dst, ways.clone(), &WayOverwritePolicy::Append)?;
        add_ways_to_graph(self, src, dst, ways, &WayOverwritePolicy::Append)?;
        Ok(())
    }

    /// updates a way in the graph, or fails if the way is missing.
    ///
    /// # Arguments
    /// * `src` - source node
    /// * `dst` - destination node
    /// * `index` - index in the multiedge set to update
    /// * `way` - replacement way data
    pub fn update_way(
        &mut self,
        src: &OsmNodeId,
        dst: &OsmNodeId,
        index: usize,
        way: OsmWayData,
    ) -> Result<(), OsmError> {
        add_ways_to_graph(
            self,
            src,
            dst,
            vec![way.clone()],
            &WayOverwritePolicy::UpdateAtIndex { index },
        )?;
        add_ways_to_graph(
            self,
            src,
            dst,
            vec![way],
            &WayOverwritePolicy::UpdateAtIndex { index },
        )?;
        Ok(())
    }

    /// replaces the set of multiedges between an od pair
    ///
    /// # Arguments
    /// * `src` - source node
    /// * `dst` - destination node
    /// * `segments` - references to OSM Ways which combine to describe
    /// * `overwrite_policy` - define the intention when existing ways are found
    pub fn replace_ways(
        &mut self,
        src: &OsmNodeId,
        dst: &OsmNodeId,
        ways: Vec<OsmWayData>,
    ) -> Result<(), OsmError> {
        add_ways_to_graph(self, src, dst, ways.clone(), &WayOverwritePolicy::Replace)?;
        add_ways_to_graph(self, src, dst, ways, &WayOverwritePolicy::Replace)?;
        Ok(())
    }

    // /// adds a new way or replaces an existing way after comparing
    // /// highway tags
    // ///
    // /// # Arguments
    // /// * `src` - source node
    // /// * `dst` - destination node
    // /// * `way` - way to add
    // pub fn add_best_way(
    //     &mut self,
    //     src: &OsmNodeId,
    //     dst: &OsmNodeId,
    //     way: OsmWayData,
    // ) -> Result<(), OsmError> {
    //     add_way_to_graph(
    //         self,
    //         src,
    //         dst,
    //         way.clone(),
    //         &WayOverwritePolicy::CompareExistingHighwayTag,
    //     )?;
    //     add_way_to_graph(
    //         self,
    //         src,
    //         dst,
    //         way,
    //         &WayOverwritePolicy::CompareExistingHighwayTag,
    //     )?;
    //     Ok(())
    // }

    // /// safely removes a way from the graph by removing all traces of it from
    // /// the adjacency lists and the ways collection. used during graph simplification.
    // pub fn remove_way_adjacencies(&mut self, way_id: OsmWayId) -> Result<(), OsmError> {
    //     let way: OsmWayData = self
    //         .ways
    //         .get(&way_id)
    //         .ok_or_else(|| {
    //             OsmError::GraphModificationError(format!(
    //                 "attempting to remove way {} not found in graph",
    //                 way_id
    //             ))
    //         })?
    //         .clone();

    //     // for each node -> node pair in the way, remove the corresponding entry in the fwd collection
    //     for (src, dst) in way.nodes.iter().tuple_windows() {
    //         remove_segment_from_adjacency(&mut self.adj, src, dst, Dir::Forward, true)?;
    //         remove_segment_from_adjacency(&mut self.adj, src, dst, Dir::Reverse, true)?;
    //     }

    //     Ok(())
    // }

    /// removes an OsmNodeData entry for the given OsmNodeId. has no effect on the
    /// adjacency matrix.
    pub fn remove_node_data(&mut self, node_id: &OsmNodeId) -> Result<(), OsmError> {
        match self.nodes.remove(node_id) {
            Some(_) => Ok(()),
            None => Err(OsmError::GraphMissingNodeId(*node_id)),
        }
    }

    /// disconnects a node from the adjacency list. has no effect on the OsmNodeData.
    pub fn disconnect_node(
        &mut self,
        node_id: &OsmNodeId,
        fail_if_missing: bool,
    ) -> Result<(), OsmError> {
        let out_neighbors = &self.get_out_neighbors(node_id);
        let in_neighbors = &self.get_in_neighbors(node_id);
        if let Some(ons) = out_neighbors {
            for dst in ons.iter() {
                self.remove_way(node_id, dst, fail_if_missing)?;
            }
        }
        if let Some(ins) = in_neighbors {
            for src in ins.iter() {
                self.remove_way(src, node_id, fail_if_missing)?;
            }
        }
        Ok(())
    }

    /// disconnects a node from the adjacency list and gives it a new (negated) id.
    /// this node has become invalid and subsumed by a consolidated node.
    /// its data is retained for completeness and debugging purposes only.
    /// relies on other methods to update node and segment counts in the graph.
    pub fn retire_node(
        &mut self,
        old_node_id: &OsmNodeId,
        fail_if_missing: bool,
    ) -> Result<(), OsmError> {
        let new_node_id = OsmNodeId(-old_node_id.0);
        let mut node = self
            .nodes
            .get(old_node_id)
            .ok_or(OsmError::GraphMissingNodeId(*old_node_id))?
            .clone();
        node.osmid = new_node_id;

        // remove all segments connected

        // self.disconnect_node(old_node_id, fail_if_missing)?;  // removing node does this
        self.disconnect_node(old_node_id, fail_if_missing)?;
        self.remove_node_data(old_node_id)?;
        self.add_node_only(node.clone())?;
        Ok(())
    }

    /// removes a directed segment between two nodes, which should exist twice in the graph, once
    /// for each adjacency direction.
    ///
    /// note: it is assumed that (src, dst) is an existing adjacency and so this method fails
    /// when one of the following occurs:
    ///   - the adjacency entry (src, Forward) -> (dst) does not exist
    ///   - the adjacency entry (dst, Reverse) -> (src) does not exist (this is the same link)
    ///   - if, after the operation, src or dst is disconnected yet removing it from the adjacency list fails
    pub fn remove_way(
        &mut self,
        src: &OsmNodeId,
        dst: &OsmNodeId,
        fail_if_missing: bool,
    ) -> Result<(), OsmError> {
        remove_way_from_adjacency(self, src, dst, Dir::Forward, fail_if_missing)?;
        remove_way_from_adjacency(self, src, dst, Dir::Reverse, fail_if_missing)?;
        self.clear_adjacency_entry_if_disconnected(src, fail_if_missing)?;
        self.clear_adjacency_entry_if_disconnected(dst, fail_if_missing)?;
        // self.n_segments -= 1;
        Ok(())
    }

    // /// update the adjacencies to reflect some simplified path
    // ///
    // /// # Arguments
    // ///
    // /// * `sp` - data describing the path to simplify
    // pub fn simplify_path(&mut self, sp: &SimplifiedPath) -> Result<(), OsmError> {
    //     log::debug!(
    //         "simplify {}, a path with {} nodes",
    //         sp.seg.way_id,
    //         sp.path.len()
    //     );
    //     match sp.path.len() {
    //         0 => {
    //             return Err(OsmError::GraphSimplificationError(String::from(
    //                 "simplify path called with empty path",
    //             )))
    //         }
    //         1 => {
    //             return Err(OsmError::GraphSimplificationError(String::from(
    //                 "simplify path called with invalid path that only contains one node",
    //             )))
    //         }
    //         2 => return Ok(()),
    //         _ => {}
    //     }

    //     let src = sp.path.first().ok_or_else(|| {
    //         OsmError::InternalError(String::from("non-empty path has no source node"))
    //     })?;
    //     let dst = sp.path.last().ok_or_else(|| {
    //         OsmError::InternalError(String::from("non-empty path has no destination node"))
    //     })?;
    //     log::debug!(
    //         "  source coordinate: {}",
    //         self.get_node_data(src)
    //             .unwrap()
    //             .get_point()
    //             .to_wkt()
    //             .to_string(),
    //     );
    //     let node_pairs = sp.path.iter().tuple_windows();
    //     log::debug!(
    //         "  removing {}",
    //         sp.path.iter().map(|n| format!("({})", n)).join("->")
    //     );
    //     for (u, v) in node_pairs {
    //         // assuming here that it's possible for a segment to be removed by more than one path,
    //         // hence fail_if_missing=false.
    //         self.remove_segment(u, v)?;
    //     }
    //     log::debug!("  adding [({})->({})]", src, dst);
    //     self.add_segment(*src, *dst, sp.seg.clone())?;
    //     Ok(())
    // }

    /// creates an entry for each direction in the adjacency list for this node id
    fn intialize_adjacency(&mut self, node_id: &OsmNodeId) -> Result<(), OsmError> {
        init_adjacency(&mut self.adj, node_id, Dir::Forward)?;
        init_adjacency(&mut self.adj, node_id, Dir::Reverse)?;
        Ok(())
    }

    /// removes a node from the adjacency list
    fn remove_adjacency_list_entry(
        &mut self,
        node_id: &OsmNodeId,
        fail_if_missing: bool,
    ) -> Result<(), OsmError> {
        remove_adjacency_list_entry(&mut self.adj, node_id, Dir::Forward, fail_if_missing)?;
        remove_adjacency_list_entry(&mut self.adj, node_id, Dir::Reverse, fail_if_missing)?;
        Ok(())
    }

    /// clears the entry for a node in the graph if it is fully disconnected.
    fn clear_adjacency_entry_if_disconnected(
        &mut self,
        node_id: &OsmNodeId,
        fail_if_missing: bool,
    ) -> Result<(), OsmError> {
        match self.node_degree(node_id) {
            Some(0) => self.remove_adjacency_list_entry(node_id, fail_if_missing),
            None if fail_if_missing => Err(OsmError::AdjacencyRemovalError(
                *node_id,
                String::from("node not present in adjacency list"),
            )),
            _ => Ok(()),
        }
    }
}

/// puts a hashmap in the adjacency list for some node id and direction
fn init_adjacency(adj: &mut AdjacencyList3, node_id: &OsmNodeId, dir: Dir) -> Result<(), OsmError> {
    match adj.insert((*node_id, dir), HashSet::new()) {
        Some(_) => Err(OsmError::InvalidOsmData(format!(
            "attempting to insert node {node_id} already present in {dir} adjacencies"
        ))),
        None => Ok(()),
    }
}

fn remove_adjacency_list_entry(
    adj: &mut AdjacencyList3,
    node_id: &OsmNodeId,
    dir: Dir,
    fail_if_missing: bool,
) -> Result<(), OsmError> {
    match adj.remove(&(*node_id, dir)) {
        Some(_) => Ok(()),
        None if fail_if_missing => Err(OsmError::AdjacencyRemovalError(
            *node_id,
            format!("no {dir} adjacency to remove for this node"),
        )),
        None => Ok(()),
    }
}

/// adds a connection between two nodes in some direction to the adjacency list.
/// also serves as an update method (with overwrite=true)
fn add_ways_to_graph(
    graph: &mut OsmGraph,
    src: &OsmNodeId,
    dst: &OsmNodeId,
    ways: Vec<OsmWayData>,
    overwrite_policy: &WayOverwritePolicy,
) -> Result<(), OsmError> {
    use WayOverwritePolicy as P;

    // determine what kind of update to perform based on the combination of
    // policy, adjacencies, and incoming ways
    let action = (
        overwrite_policy,
        graph.ways.get_mut(&(*src, *dst)),
        &ways.as_slice(),
    );
    match action {
        // calling this method with an empty ways collection is an error
        (_, _, []) => {
            return Err(OsmError::InternalError(
                "add ways to graph called with no ways to add".to_string(),
            ))
        }
        // append to None or replace both simply insert without checking
        (P::Append, None, _) | (P::Replace, _, _) => {
            let _ = graph.ways.insert((*src, *dst), ways);
        }
        // append to Some extends the existing multiedge collection
        (P::Append, Some(prev_ways), _) => {
            prev_ways.extend(ways);
        }

        // update at index but the index is too high
        (P::UpdateAtIndex { index }, Some(ways), _) if *index >= ways.len() => {
            return Err(OsmError::GraphModificationError(format!(
                "attempting to modify way ({})-[..]->({}) at way index {} which exceeds the size of the multiedge collection {}",
                src, dst, index, ways.len()
            )));
        }
        // update at index with a valid index and correctly called with a single way to update
        (P::UpdateAtIndex { index }, Some(ways), [way]) => {
            ways.insert(*index, way.clone());
        }
        // update at index with no multiedges is an error
        (P::UpdateAtIndex { index }, None, [way]) => {
            let way_ids = ways.iter().map(|w| w.osmid).join(",");
            return Err(OsmError::GraphModificationError(format!(
                "attempting to modify way ({})-[{}]->({}) at way index {} but it does not exist",
                src, way.osmid, dst, index
            )));
        }
        (P::UpdateAtIndex { index }, None, _) => {
            return Err(OsmError::InternalError(format!(
                "add ways to graph called but multiedge is empty, has no index index {index}"
            )))
        }
        (P::UpdateAtIndex { index }, Some(_), _) => {
            return Err(OsmError::InternalError(format!(
            "add ways to graph called but cannot replace a single way at index {} with {} new ways",
            index,
            ways.len()
        )))
        }
    }

    if let Some(neighbors) = graph.adj.get_mut(&(*src, Dir::Forward)) {
        if !neighbors.contains(dst) {
            let _ = neighbors.insert(*dst);
        }
    }
    if let Some(neighbors) = graph.adj.get_mut(&(*dst, Dir::Reverse)) {
        if !neighbors.contains(src) {
            let _ = neighbors.insert(*src);
        }
    }

    Ok(())
}

/// removes a way between two nodes in some direction to the adjacency list. accounts for
/// flipping the direction depending on the Direction of the pair, so,
fn remove_way_from_adjacency(
    graph: &mut OsmGraph,
    src: &OsmNodeId,
    dst: &OsmNodeId,
    dir: Dir,
    fail_if_missing: bool,
) -> Result<(), OsmError> {
    let (outer, inner) = match dir {
        Dir::Forward => (*src, *dst),
        Dir::Reverse => (*dst, *src),
    };
    if let Some(adjacencies) = graph.adj.get_mut(&(outer, dir)) {
        let was_present = adjacencies.remove(dst);
        if !was_present && fail_if_missing {
            return Err(OsmError::GraphSimplificationError(format!(
                "attempting to remove {dir} adjacency ({src}) -> ({dst}) that does not exist"
            )));
        }
    }
    Ok(())
}

/// helper to update the graph edges incident to a new consolidated node.
///
/// # Arguments
/// * `new_node_id` - id replacing the src/dst node id for this way
/// * `node_ids`    - ids that are being consolidated
/// * `graph`       - graph to modify
/// * `dir`         - direction in adjacency list to find the ways to modify
fn update_incident_way_data(
    new_node_id: OsmNodeId,
    node_ids: &[OsmNodeId],
    graph: &mut OsmGraph,
    dir: Dir,
) -> Result<(), OsmError> {
    // find the ways that will be impacted by consolidation
    let remove_nodes: HashSet<&OsmNodeId> = node_ids.iter().collect();
    let updated = node_ids
        .iter()
        .map(|src| {
            let adj = graph.get_neighbors(src, dir).unwrap_or_default();
            let updated_ways = adj
                .iter()
                .map(|dst| {
                    let ways = graph.get_ways_from_od(src, dst)?;
                    let ways_updated = ways
                        .iter()
                        .enumerate()
                        .map(|(index, way)| {
                            let mut updated = way.clone();
                            updated.nodes.retain(|n| !remove_nodes.contains(n));

                            // insert the new node in the correct position along this way
                            match dir {
                                Dir::Forward => updated.nodes.insert(0, new_node_id),
                                Dir::Reverse => updated.nodes.push(new_node_id),
                            }
                            (*src, *dst, index, updated)
                        })
                        .collect_vec();
                    Ok(ways_updated)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(updated_ways)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect_vec();

    for ways in updated.into_iter() {
        for (src, dst, index, way) in ways.into_iter() {
            graph.update_way(&src, &dst, index, way)?;
        }
    }

    // for node_id in node_ids.iter() {
    //     for way in graph.get_adjacencies(node_id, dir)?.values() {
    //         if way.nodes.is_empty() {
    //             return Err(OsmError::InternalError(format!(
    //                 "way {} has empty node list",
    //                 way.osmid
    //             )));
    //         }

    //         // remove consolidated nodes from the Way nodelist, they are becoming a single point
    //         way.nodes.retain(|n| !remove_nodes.contains(n));

    //         // insert the new node in the correct position along this way
    //         match dir {
    //             Dir::Forward => way.nodes.insert(0, new_node_id),
    //             Dir::Reverse => way.nodes.push(new_node_id),
    //         }
    //     }
    // }

    // for way_id in updated_way_ids.iter() {
    //     if way.nodes.is_empty() {
    //         return Err(OsmError::InternalError(format!(
    //             "way {} has empty node list",
    //             way_id
    //         )));
    //     }

    //     // remove consolidated nodes from the Way nodelist, they are becoming a single point
    //     way.nodes.retain(|n| !remove_nodes.contains(n));

    //     // insert the new node in the correct position along this way
    //     match dir {
    //         Dir::Forward => way.nodes.insert(0, new_node_id),
    //         Dir::Reverse => way.nodes.push(new_node_id),
    //     }
    // }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::OsmGraph;
    use crate::model::osm::graph::{
        osm_node_data::OsmNodeData, osm_way_data::OsmWayData, AdjacencyDirection, OsmNodeId,
        OsmWayId,
    };

    // #[test]
    // fn test_add_and_remove() {
    //     // setup
    //     let mut graph = OsmGraph::default();
    //     let mut n1 = OsmNodeData::default();
    //     let mut n2 = OsmNodeData::default();
    //     let nid1 = OsmNodeId(1);
    //     let nid2 = OsmNodeId(2);
    //     let wid1 = OsmWayId(1);
    //     n1.osmid = nid1;
    //     n1.x = 0.0;
    //     n1.y = 0.0;
    //     n2.osmid = nid2;
    //     n2.x = 1.0;
    //     n2.y = 1.0;
    //     let mut w1 = OsmWayData::default();
    //     w1.osmid = wid1;
    //     w1.nodes = vec![n1.osmid, n2.osmid];

    //     // 1. add to graph
    //     graph.add_node_and_adjacencies(n1).unwrap();
    //     graph.add_node_and_adjacencies(n2).unwrap();
    //     graph.add_way_and_adjacencies(w1).unwrap();

    //     // 2. remove way, should leave nodes untouched
    //     graph.remove_way_adjacencies(wid1).unwrap();
    //     assert_eq!(graph.nodes.len(), 2);
    //     assert_eq!(graph.ways.len(), 0);
    //     // 3. remove one node, should not impact other node
    //     graph.remove_node_adjacencies(nid1).unwrap();
    //     assert_eq!(graph.nodes.len(), 1);
    //     assert_eq!(graph.ways.len(), 0);
    //     // 4. remove other node, graph should be empty
    //     graph.remove_node_adjacencies(nid2).unwrap();
    //     assert_eq!(graph.nodes.len(), 0);
    //     assert_eq!(graph.ways.len(), 0);
    // }

    // #[test]
    // fn test_remove_connected_node() {
    //     // setup
    //     let mut graph = OsmGraph::default();
    //     let mut n1 = OsmNodeData::default();
    //     let mut n2 = OsmNodeData::default();
    //     n1.osmid = OsmNodeId(0);
    //     n1.x = 0.0;
    //     n1.y = 0.0;
    //     n2.osmid = OsmNodeId(1);
    //     n2.x = 1.0;
    //     n2.y = 1.0;
    //     let mut w1 = OsmWayData::default();
    //     w1.osmid = OsmWayId(0);
    //     w1.nodes = vec![n1.osmid, n2.osmid];

    //     // 1. add to graph
    //     graph.add_node_and_adjacencies(n1).unwrap();
    //     graph.add_node_and_adjacencies(n2).unwrap();
    //     graph.add_way_and_adjacencies(w1).unwrap();

    //     // 2. remove a node
    //     let remove_node_id = OsmNodeId(0);
    //     graph.remove_node_and_adjacencies(remove_node_id).unwrap();

    //     assert!(
    //         graph.nodes.get(&remove_node_id).is_none(),
    //         "node should have been removed"
    //     );

    //     assert_eq!(graph.ways.len(), 0, "should have removed the way also");

    //     let expected_key = (remove_node_id, AdjacencyDirection::Forward);
    //     assert!(
    //         graph.adj.get(&expected_key).is_none(),
    //         "node should be removed from adjacencies"
    //     );
    // }
}
