use super::{
    graph::{
        osm_element_filter::ElementFilter, osm_segment::OsmSegment, AdjacencyList3,
        AdjacencyListDeprecated, OsmGraph, OsmNodeId, OsmNodes, OsmWayId, OsmWays,
    },
    OsmError,
};
use crate::{
    algorithm::{buffer, connected_components},
    model::{
        feature::highway::Highway,
        osm::graph::{
            osm_node_data::OsmNodeData, osm_way_data::OsmWayData, AdjacencyDirection, AdjacencyList,
        },
    },
};
use geo::{
    line_string, point, Contains, Convert, Coord, CoordsIter, Geometry, Haversine, Intersects,
    Length, Line, MultiPolygon,
};
use geo::{Centroid, KNearestConcaveHull, Point, Scale};
use itertools::{Either, Itertools};
use kdam::{term, tqdm, Bar, BarExt};
use osmpbf::{Element, ElementReader};
use rayon::prelude::*;
use routee_compass_core::util::compact_ordered_hash_map::CompactOrderedHashMap;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};
use wkt::ToWkt;

/// an estimate of this is fine, it's used to buffer the extent of the study area
/// by a smidge so we don't prematurely truncate edges when reading the network source.
// pub const BUFFER_500M_IN_DEGREES: f32 = 0.0045000045;
pub const BUFFER_500M: f32 = 500.0;

/// reads a PBF file and stores the Ways and Nodes in lookup objects. filters out nodes
/// and ways based on the filter and extent_opt arguments:
/// - if provided, extent_opt will filter out nodes with points found outside of the extent
/// - the provided [`NetworkFilter`] filters rows based on their [`Highway`] tag
/// - ways that had their nodes removed are also removed
pub fn read_pbf(
    filepath: &str,
    filter: ElementFilter,
    extent_opt: &Option<Geometry<f32>>,
) -> Result<(OsmNodes, OsmWays), OsmError> {
    let fp = Path::new(filepath);
    let reader = ElementReader::from_path(fp).map_err(|e| OsmError::PbfLibError { source: e })?;

    // if provided an extent geometry, it is buffered by 500 meters, and then used as a node filter function
    let ext_buffered_opt = match extent_opt {
        Some(g) => {
            log::info!("buffering extent for initial download filtering");
            let g_buf = buffer::scale_exterior(g, BUFFER_500M).map_err(|e| {
                OsmError::ConfigurationError(format!("failure buffering extent: {e}"))
            })?;
            Ok(Some(g_buf))
        }
        None => Ok(None),
    }?;
    let within_extent_fn = Box::new(|g: &OsmNodeData| match ext_buffered_opt.as_ref() {
        Some(ext) => g.intersects(ext),
        None => true,
    });

    term::hide_cursor().map_err(|e| OsmError::InternalError(e.to_string()))?;
    let mut reader_bar = Bar::builder()
        .desc(filepath)
        .position(0)
        .unit(" rows")
        .unit_scale(true)
        .build()
        .map_err(OsmError::InternalError)?;
    let mut nodes_bar = Bar::builder()
        .desc("nodes retained")
        .position(1)
        .build()
        .map_err(OsmError::InternalError)?;
    let mut ways_bar = Bar::builder()
        .desc("ways retained")
        .position(2)
        .build()
        .map_err(OsmError::InternalError)?;

    let mut nodes_map: OsmNodes = HashMap::default();
    let mut ways_map: OsmWays = HashMap::default();
    let mut nodes_visited: usize = 0;
    let mut ways_visited: usize = 0;
    // pull in all Node and Way rows. along the way, track which NodeOsmids are
    // present in the ways that have been accepted by the filter function.
    reader
        .for_each(|e| {
            let valid_element = filter.accept(&e);
            match e {
                Element::Node(node) if !valid_element => {
                    nodes_visited += 1;
                }
                Element::Node(node) => {
                    nodes_visited += 1;
                    if node.id() == 0 {
                        log::warn!(
                            "node missing OSMID at ({},{}) ignored",
                            node.lon(),
                            node.lat()
                        );
                    } else {
                        let n = OsmNodeData::from(&node);
                        if nodes_map.contains_key(&n.osmid) {
                            log::warn!(
                                "node with OSMID {} occurs more than once in this file",
                                n.osmid
                            );
                        }
                        if within_extent_fn(&n) {
                            let _ = nodes_bar.update(1);
                            nodes_map.insert(n.osmid, n);
                        }
                    }
                }
                Element::DenseNode(dense) if !valid_element => {
                    nodes_visited += 1;
                }
                Element::DenseNode(dense) => {
                    nodes_visited += 1;
                    // from documentation on DenseNode:
                    // So, if you want to [pattern match on] `Node`, you also likely want to match [`DenseNode`].
                    let n = OsmNodeData::from(&dense);
                    if nodes_map.contains_key(&n.osmid) {
                        log::warn!(
                            "node with OSMID {} occurs more than once in this file",
                            n.osmid
                        );
                    }
                    if within_extent_fn(&n) {
                        let _ = nodes_bar.update(1);
                        nodes_map.insert(n.osmid, n);
                    }
                }
                Element::Way(way) if !valid_element => {
                    ways_visited += 1;
                }
                Element::Way(way) => {
                    ways_visited += 1;
                    let w = OsmWayData::new(&way);
                    if ways_map.contains_key(&w.osmid) {
                        log::warn!(
                            "way with OSMID {} occurs more than once in this file",
                            w.osmid
                        );
                    }
                    let _ = ways_bar.update(1);
                    ways_map.insert(w.osmid, w);
                }
                Element::Relation(_) => {}
            }
            let _ = reader_bar.update(1);
        })
        .map_err(|e| OsmError::PbfLibError { source: e })?;

    // close the 3 nested progress bars
    eprintln!();
    eprintln!();
    eprintln!();
    term::show_cursor().map_err(|e| OsmError::InternalError(e.to_string()))?;

    if nodes_map.is_empty() {
        return Err(OsmError::NoNodesFound);
    }
    if ways_map.is_empty() {
        return Err(OsmError::NoWaysFound);
    }

    // // # G.add_nodes_from(nodes.items())
    // // # _add_paths(G, paths.values(), bidirectional)  # where paths is Map[Osmid, Way]

    if extent_opt.is_some() {
        // we may have filtered nodes along the way, so here we must remove the ways that were attached to them
        let mut disconnected_ways = vec![];
        let mut connected_nodes: HashSet<OsmNodeId> = HashSet::new();
        let find_disconnected_ways_iter = tqdm!(
            ways_map.values(),
            desc = "find ways disconnected by extent filtering",
            total = ways_map.len()
        );

        // remove this way if it contains any nodes which are not present in our nodes_map
        // as they may lie outside of the study region.
        for way in find_disconnected_ways_iter {
            let disconnected_way = way.nodes.iter().any(|n_id| !nodes_map.contains_key(n_id));
            if disconnected_way {
                disconnected_ways.push(way.osmid);
            } else {
                for node_id in way.nodes.iter() {
                    // while we could just "insert" only, this test avoids unnecessary cloning
                    if !connected_nodes.contains(node_id) {
                        connected_nodes.insert(*node_id);
                    }
                }
            }
            // let src_node_id = way.src_node_id()?;
            // let dst_node_id = way.dst_node_id()?;
            // let disconnected =
            //     !nodes_map.contains_key(&src_node_id) || !nodes_map.contains_key(&dst_node_id);
            // if disconnected {
            //     disconnected_ways.push(way.osmid);
            // } else {
            //     connected_nodes.insert(src_node_id);
            //     connected_nodes.insert(dst_node_id);
            // }
        }
        eprintln!();

        let remove_disconnected_ways_iter = tqdm!(
            disconnected_ways.iter(),
            desc = "remove ways disconnected by extent filtering",
            total = disconnected_ways.len()
        );
        for way_id in remove_disconnected_ways_iter {
            ways_map.remove(way_id);
        }
        eprintln!();

        // finally, remove nodes that became detached after way filtering
        let mut disconnected_nodes = vec![];
        let find_disconnected_node_iter = tqdm!(
            nodes_map.values(),
            desc = "find nodes disconnected by extent filtering",
            total = nodes_map.len()
        );
        for node in find_disconnected_node_iter {
            let disconnected = !connected_nodes.contains(&node.osmid);
            if disconnected {
                disconnected_nodes.push(node.osmid);
            }
        }
        eprintln!();

        let remove_disconnected_node_iter = tqdm!(
            disconnected_nodes.iter(),
            desc = "remove nodes disconnected by extent filtering",
            total = disconnected_nodes.len()
        );
        for node_id in remove_disconnected_node_iter {
            nodes_map.remove(node_id);
        }
        eprintln!();
    }
    // // we may have filtered nodes along the way, so here we must remove the ways that were attached to them
    // let mut disconnected_ways = vec![];
    // let mut connected_nodes: HashSet<OsmNodeId> = HashSet::new();
    // let find_disconnected_ways_iter = tqdm!(
    //     ways_map.values(),
    //     desc = "find ways with missing nodes",
    //     total = ways_map.len()
    // );

    // // remove this way if it contains any nodes which are not present in our nodes_map
    // // as they may lie outside of the study region.
    // for way in find_disconnected_ways_iter {
    //     let disconnected_way = way.nodes.iter().any(|n_id| !nodes_map.contains_key(n_id));
    //     if disconnected_way {
    //         disconnected_ways.push(way.osmid);
    //     } else {
    //         for node_id in way.nodes.iter() {
    //             // while we could just "insert" only, this test avoids unnecessary copies
    //             if !connected_nodes.contains(node_id) {
    //                 connected_nodes.insert(*node_id);
    //             }
    //         }
    //     }
    //     // let src_node_id = way.src_node_id()?;
    //     // let dst_node_id = way.dst_node_id()?;
    //     // let disconnected =
    //     //     !nodes_map.contains_key(&src_node_id) || !nodes_map.contains_key(&dst_node_id);
    //     // if disconnected {
    //     //     disconnected_ways.push(way.osmid);
    //     // } else {
    //     //     connected_nodes.insert(src_node_id);
    //     //     connected_nodes.insert(dst_node_id);
    //     // }
    // }
    // eprintln!();

    // let remove_disconnected_ways_iter = tqdm!(
    //     disconnected_ways.iter(),
    //     desc = "remove disconnected ways",
    //     total = disconnected_ways.len()
    // );
    // for way_id in remove_disconnected_ways_iter {
    //     ways_map.remove(way_id);
    // }

    // // finally, remove nodes that became detached after way filtering
    // let mut disconnected_nodes = vec![];
    // let find_disconnected_node_iter = tqdm!(
    //     nodes_map.values(),
    //     desc = "find disconnected nodes",
    //     total = nodes_map.len()
    // );
    // for node in find_disconnected_node_iter {
    //     let disconnected = !connected_nodes.contains(&node.osmid);
    //     if disconnected {
    //         disconnected_nodes.push(node.osmid);
    //     }
    // }
    // eprintln!();
    // let remove_disconnected_node_iter = tqdm!(
    //     disconnected_nodes.iter(),
    //     desc = "remove disconnected nodes",
    //     total = disconnected_nodes.len()
    // );
    // for node_id in remove_disconnected_node_iter {
    //     nodes_map.remove(node_id);
    // }
    // eprintln!();

    log::info!(
        "{} ways and {} nodes collected from OSM pbf resource.",
        ways_map.len(),
        nodes_map.len(),
        // nodes_visited,
        // ways_visited
    );
    Ok((nodes_map, ways_map))
}

// pub fn build_adjacencies(ways_map: &OsmWays) -> Result<AdjacencyList3, OsmError> {
//     let mut adj: AdjacencyList3 = HashMap::new();

//     let ways_iter = tqdm!(
//         ways_map.values(),
//         total = ways_map.len(),
//         desc = "building adjacency list"
//     );
//     for way in ways_iter {
//         // # extract/remove the ordered list of nodes from this path element so
//         // # we don't add it as a superfluous attribute to the edge later
//         let mut nodes = way.nodes.clone();

//         // # reverse the order of nodes in the path if this path is both one-way
//         // # and only allows travel in the opposite direction of nodes' order
//         let oneway = way.is_one_way();
//         let reverse = way.is_reverse();
//         if oneway && reverse {
//             nodes.reverse();
//         }

//         // # set the oneway attribute, but only if when not forcing all edges to
//         // # oneway with the all_oneway setting. With the all_oneway setting, you
//         // # want to preserve the original OSM oneway attribute for later clarity
//         // --> SKIPPED since we don't support this

//         // # zip path nodes to get (u, v) tuples like [(0,1), (1,2), (2,3)].
//         // # add all the edge tuples and give them the path's tag:value attrs
//         for (src_id, dst_id) in nodes.iter().tuple_windows() {
//             // # G.add_edges_from(edges, **path)
//             // insert forward-oriented segment
//             let mut fwd_entries = adj
//                 .entry((*src_id, AdjacencyDirection::Forward))
//                 .or_insert(HashMap::new());

//             if !oneway {
//                 // # G.add_edges_from((v, u) for u, v in edges], **path)
//                 // insert reverse-oriented segment
//                 let mut rev_entries = adj
//                     .entry((*dst_id, AdjacencyDirection::Reverse))
//                     .or_insert(HashMap::new());
//                 insert_op2(src_id, way, false, rev_entries)?;
//             }
//         }
//     }
//     eprintln!();

//     log::info!(
//         "adjacency list has {} nodes, {} segments",
//         adj.len(),
//         adj.values().map(|adj| adj.len()).sum::<usize>()
//     );
//     Ok(adj)
// }

pub fn build_adjacencies_2(ways_map: &OsmWays) -> Result<AdjacencyList, OsmError> {
    let mut adj: AdjacencyList = HashMap::new();

    let ways_iter = tqdm!(
        ways_map.values(),
        total = ways_map.len(),
        desc = "building adjacency list"
    );
    for way in ways_iter {
        // # extract/remove the ordered list of nodes from this path element so
        // # we don't add it as a superfluous attribute to the edge later
        let mut nodes = way.nodes.clone();

        // # reverse the order of nodes in the path if this path is both one-way
        // # and only allows travel in the opposite direction of nodes' order
        let oneway = way.is_one_way();
        let reverse = way.is_reverse();
        if oneway && reverse {
            nodes.reverse();
        }

        // # set the oneway attribute, but only if when not forcing all edges to
        // # oneway with the all_oneway setting. With the all_oneway setting, you
        // # want to preserve the original OSM oneway attribute for later clarity
        // --> SKIPPED since we don't support this

        // # zip path nodes to get (u, v) tuples like [(0,1), (1,2), (2,3)].
        // # add all the edge tuples and give them the path's tag:value attrs
        for (src_id, dst_id) in nodes.iter().tuple_windows() {
            // # G.add_edges_from(edges, **path)
            // insert forward-oriented segment
            let mut fwd_segs = adj
                .entry((*src_id, AdjacencyDirection::Forward))
                .or_default();
            insert_op(dst_id, way, oneway, fwd_segs)?;

            if !oneway {
                // # G.add_edges_from((v, u) for u, v in edges], **path)
                // insert reverse-oriented segment
                let mut rev_segs = adj
                    .entry((*dst_id, AdjacencyDirection::Reverse))
                    .or_default();
                insert_op(src_id, way, false, rev_segs)?;
            }
        }
    }
    eprintln!();

    log::info!(
        "adjacency list has {} nodes, {} segments",
        adj.len(),
        adj.values().map(|adj| adj.len()).sum::<usize>()
    );
    Ok(adj)
}

/// inserts adjacencies into the adjacency list during initialization.
/// not intended for simplified graphs, as this maintains the invariant that there
/// exists one way between each (src, dst) node pair.
fn insert_op(
    target_id: &OsmNodeId,
    way: &OsmWayData,
    is_oneway: bool,
    entry: &mut HashMap<OsmNodeId, Vec<OsmSegment>>,
) -> Result<(), OsmError> {
    let highway = match &way.highway {
        Some(h) => {
            let highway = h.parse::<Highway>().map_err(|e| {
                OsmError::InvalidOsmData(format!("unknown highway tag '{h}': {e}"))
            })?;
            Some(highway)
        }
        None => None,
    };
    let next_segment = OsmSegment::new(way.osmid, highway, is_oneway);

    // from osmnx.graph.simplification:378:
    //      # ...if multiple edges exist between
    //      # them (see above), we retain only one in the simplified graph
    //      # We can't assume that there exists an edge from u to v
    //      # with key=0, so we get a list of all edges from u to v
    //      # and just take the first one.
    // aka, OSMNX randomly picks a tiebreaker. here let's pick the option that
    // is higher in the road network hierarchy if possible (defaulting to random).
    match entry.get(target_id) {
        Some(prev_segments) => {
            match &prev_segments.as_slice() {
                [prev_segment] if &next_segment <= prev_segment => {
                    let _ = entry.insert(*target_id, vec![next_segment]);
                    // newly-inserted segment has greater highway authority, replace the old one
                }
                _ => {} // NOOP
            }
        }
        None => {
            let _ = entry.insert(*target_id, vec![next_segment]);
        }
    }
    Ok(())
}

fn insert_op2(
    target_id: &OsmNodeId,
    way: &OsmWayData,
    is_oneway: bool,
    entry: &mut HashMap<OsmNodeId, OsmWayData>,
) -> Result<(), OsmError> {
    // determine if we should insert this new way, depending on whether there is an
    // existing way in this entry, and if so, how their highway tags compare.
    // rjf 2025-03-06: should we actually combine these instead?
    let insert_way = match (
        way.get_highway(),
        entry.get(target_id).map(|w| w.get_highway()),
    ) {
        (Ok(None), _) => false,
        (Ok(Some(highway)), None) => true,
        (Ok(Some(new_highway)), Some(Ok(Some(old_highway)))) => new_highway < old_highway,
        (_, Some(Ok(None))) => {
            return Err(OsmError::InternalError(String::from(
                "existing way in the graph has no highway tag",
            )))
        }
        (Ok(_), Some(Err(e))) => {
            return Err(OsmError::InternalError(format!(
                "existing way in the graph has an invalid highway tag '{}': {}",
                way.highway.clone().unwrap_or_default(),
                e
            )))
        }
        (Err(e), None) => return Err(OsmError::InvalidOsmData(format!("{e}"))),
        (Err(e), Some(_)) => return Err(OsmError::InvalidOsmData(format!("{e}"))),
    };

    let highway = match &way.highway {
        Some(h) => {
            let highway = h.parse::<Highway>().map_err(|e| {
                OsmError::InvalidOsmData(format!("unknown highway tag '{h}': {e}"))
            })?;
            Some(highway)
        }
        None => None,
    };

    if insert_way {
        let _ = entry.insert(*target_id, way.clone());
    }

    Ok(())
}

pub fn build_rev_adjacency_list(fwd: &AdjacencyListDeprecated) -> AdjacencyListDeprecated {
    let mut rev: AdjacencyListDeprecated = HashMap::new();
    let segments_iter = tqdm!(
        fwd.iter(),
        desc = "create reverse adjacency list",
        total = fwd.len()
    );
    for (src, edges) in segments_iter {
        for (dst, edge) in edges.iter() {
            match rev.get_mut(dst) {
                Some(rev_edges) => {
                    rev_edges.insert(*src, edge.clone());
                }
                None => {
                    rev.insert(*dst, HashMap::from([(*src, edge.clone())]));
                }
            }
        }
    }
    rev
}

// #[cfg(test)]
// mod tests {
//     use super::reverse_segments;
//     use crate::model::osm::graph::{osm_segment::OsmSegment, AdjacencyList};
//     use std::collections::HashMap;

//     #[test]
//     fn test_rev() {
//         let fwd: AdjacencyList = HashMap::from([
//             (0, HashMap::from([(1, OsmSegment::new(100, None, false))])),
//             (1, HashMap::from([(0, OsmSegment::new(101, None, false))])),
//         ]);
//         let rev = reverse_segments(&fwd);
//         assert_eq!(
//             fwd.get(&0).map(|edges| edges.get(&1).cloned()),
//             rev.get(&1).map(|edges| edges.get(&0).cloned())
//         )
//     }
// }
