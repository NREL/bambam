use super::*;
use crate::algorithm::*;
use crate::model::osm::graph::AdjacencyDirection;
use crate::model::osm::graph::OsmNodeData;
use crate::model::osm::graph::OsmWayData;
use crate::model::osm::graph::{fill_value_lookup::FillValueLookup, OsmGraph, OsmNodeId};
use crate::model::osm::graph::{AdjacencyListDeprecated, OsmWayId};
use crate::model::osm::OsmError;
use clustering::ClusteredIntersections;
use geo::{BooleanOps, BoundingRect, Intersects, Polygon, RemoveRepeatedPoints};
use geo::{Coord, Geometry, Haversine, Length, LineString, MultiPolygon};
use itertools::Itertools;
use kdam::{tqdm, Bar, BarExt};
use rayon::prelude::*;
use routee_compass_core::model::{
    network::{Edge, Vertex},
    unit::{AsF64, Distance, DistanceUnit, Grade, Speed, SpeedUnit},
};
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, RTreeObject};
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
use wkt::ToWkt;

// pub type GeometryIndex = usize;
// pub type ClusterLabel = usize;

/// implements osmnx.simplification.consolidate_intersections with dead_ends=False, rebuild_graph=True,
/// reconnect_edges=True for the given distance tolerance.
/// comments describing the logic for this function is taken directly from osmnx's
/// osmnx.simplification._consolidate_intersections_rebuild_graph function.
///
/// # Arguments
///
/// * `graph`      - the original graph data from the .pbf file
/// * `simplified` - the result of graph simplification that omits unneccesary edge segmentation
/// * `tolerance`  - edge-connected simplified endpoints within this distance threshold are merged
///                  into a new graph vertex by their centroid
/// * `ignore_osm_parsing_errors` - if true, do not fail if a maxspeed or other attribute is not
///                                 valid wrt the OpenStreetMaps documentation
pub fn consolidate_graph(
    graph: &mut OsmGraph,
    tolerance: (Distance, DistanceUnit),
    ignore_osm_parsing_errors: bool,
) -> Result<(), OsmError> {
    // STEP 1
    // buffer nodes to passed-in distance and merge overlaps. turn merged nodes
    // into gdf and get centroids of each cluster as x, y.

    log::info!("buffering with tolerance {:?}", tolerance);
    let node_geometries = buffer_nodes(graph, tolerance)?;

    // STEP 2
    // attach each node to its cluster of merged nodes. first get the original
    // graph's node points then spatial join to give each node the label of
    // cluster it's within. make cluster labels type string.
    let mut rtree: RTree<ClusteredIntersections> =
        clustering::build(&node_geometries).map_err(|e| {
            OsmError::GraphConsolidationError(format!(
                "failure building geometry intersection graph: {}",
                e
            ))
        })?;

    // DEBUG: before we "Drain" the tree
    serde_json::to_writer(
        File::create("debug_nodes.json").unwrap(),
        &serde_json::to_value(node_geometries.iter().enumerate().collect_vec()).unwrap(),
    );

    // return just the clusters. sorted for improved determinism.
    let clusters: Vec<Vec<OsmNodeId>> = rtree
        .drain()
        .map(|obj| obj.data.ids())
        .sorted()
        .collect_vec();
    let sum_conn = clusters.iter().map(|s| s.len() as f64).sum::<f64>();
    let avg_conn = sum_conn / clusters.len() as f64;
    log::info!(
        "spatial intersection graph has {} entries, avg {:.4} connections",
        clusters.len(),
        avg_conn
    );

    // STEP 3
    // if a cluster contains multiple components (i.e., it's not connected)
    // move each component to its own cluster (otherwise you will connect
    // nodes together that are not truly connected, e.g., nearby deadends or
    // surface streets with bridge).
    let merged = consolidate_clusters(&clusters, graph)?;
    if merged.is_empty() {
        return Err(OsmError::GraphConsolidationError(String::from(
            "merging simplified nodes resulted in 0 merged nodes",
        )));
    }

    // ok
    // 1. we found the endpoint OSMIDs
    // 2. we found which OSMIDs can be merged because they are close
    // 3. we now need to

    log::info!("produced {} merged graph nodes", merged.len());
    // serde_json::to_writer(
    //     File::create("debug_merged.json").unwrap(),
    //     &serde_json::to_value(merged.iter().enumerate().collect_vec()).unwrap(),
    // )
    // .unwrap();

    ///////////////////////////////////////////////////////////////////////////////////
    // starting here, OSMNX has the trouble of coming back around to a valid NetworkX /
    // graph dataset with expected OSMNX attributes. in our case, our target is to    /
    // produce either a Compass Graph object or write {csv|txt}.gz files to disk.     /

    // STEP 4
    // create new empty graph and copy over misc graph data
    //   - we can probably ignore this step

    // STEP 5
    // create a new node for each cluster of merged nodes
    // regroup now that we potentially have new cluster labels from step 3
    let mut vertex_lookup: HashMap<OsmNodeId, usize> =
        HashMap::with_capacity(graph.n_connected_nodes());
    let m_iter = tqdm!(
        merged.iter().enumerate(),
        total = merged.len(),
        desc = "build Compass vertices"
    );
    // finalize merged nodes as Compass Vertex instances
    // and along the way, store a lookup from the OSMID(s) the merged node
    // was made from into the new merged vertex index.
    // let merged_vertices = m_iter
    //     .map(|(merged_vertex_id, merged_node)| {
    //         for node_osmid in merged_node.osmids.iter() {
    //             if let Some(prev) = vertex_lookup.insert(*node_osmid, merged_vertex_id) {
    //                 log::error!("{}", merged_node);
    //                 let node_pt = graph.nodes_map.get(node_osmid).map(|n| format!("({},{})", n.x, n.y)).unwrap_or_default();
    //                 let this_pt = format!("({},{})", merged_node.x, merged_node.y);
    //                 let prev_merged_opt = merged.get(prev);
    //                 let prev_pt = prev_merged_opt.map(|n| format!("({},{})", n.x, n.y)).unwrap_or_default();
    //                 let prev_ids = prev_merged_opt.map(|n| n.osmids.iter().join(",")).unwrap_or_default();

    //                 return Err(OsmError::GraphConsolidationError(format!(
    //                     "attempting to assign node with osmid {} at point {} to merged vertex {} at point {} with ids {}, but it was already inserted into merged vertex {} at point {} with ids {}",
    //                     node_osmid, node_pt, merged_vertex_id, this_pt, merged_node.osmids.iter().join(","), prev, prev_pt, prev_ids
    //                 )));
    //             }
    //         }

    //         let x = merged_node.x as f32;
    //         let y = merged_node.y as f32;
    //         Ok(Vertex::new(merged_vertex_id, x, y))
    //     })
    //     .collect::<Result<Vec<_>, _>>()?;
    // eprintln!();

    // // STEP 6
    // // create new edge from cluster to cluster for each edge in original graph
    // // STEP 7
    // // for every group of merged nodes with more than 1 node in it, extend the
    // // edge geometries to reach the new node point

    // // build a speeds lookup table from potentially sparse maxspeed data, averaged
    // // by highway class label.
    // log::info!("collecting edge attributes for distance, speed");
    // let maxspeed_cb = |r: &OsmWayData| {
    //     r.get_maxspeed(true)
    //         .map(|r_opt| {
    //             r_opt.map(|(s, su)| su.convert(&s, &SpeedUnit::KPH).as_f64())
    //         })
    //         .map_err(OsmError::GraphConsolidationError)
    // };
    // let maxspeeds_fill_lookup = FillValueLookup::new(graph, "highway", "maxspeed", maxspeed_cb)?;

    // // build Compass dataset outputs using the segment iterator of our simplified graph
    // let e_iter = tqdm!(
    //     SegmentIterator::new(simplified, &vertex_lookup).enumerate(),
    //     total = simplified.n_segments(),
    //     desc = "build Compass edge datasets"
    // );
    // let mut edges: Vec<Edge> = vec![];
    // let mut geometries: Vec<Geometry> = vec![];
    // let mut grades: Vec<Grade> = vec![];
    // let mut maxspeeds: Vec<Speed> = vec![];
    // let mut errors: Vec<OsmError> = vec![];

    // // for each new segment Compass EdgeId, along with the ids of the source Way + Nodes:
    // for (edge_id, seg_result) in e_iter {
    //     let seg = match seg_result.map_err(OsmError::GraphConsolidationError) {
    //         Ok(row) => row,
    //         Err(e) => {
    //             errors.push(e);
    //             continue;
    //         }
    //     };
    //     let way: &OsmWayData = graph.ways_map.get(&seg.way_id).ok_or_else(|| {
    //         OsmError::GraphConsolidationError(format!("way osmid {} does not exist", seg.way_id))
    //     })?;

    //     // 1. create segment LineString
    //     let coords = seg
    //         .node_osmids
    //         .iter()
    //         .map(|node_id| {
    //             let node = graph.nodes_map.get(node_id).ok_or_else(|| {
    //                 OsmError::GraphConsolidationError(format!(
    //                     "node osmid {} missing from OSM graph data",
    //                     node_id
    //                 ))
    //             })?;
    //             Ok(Coord::from((node.x, node.y)))
    //         })
    //         .collect::<Result<Vec<_>, _>>()?;
    //     let linestring = LineString::from(coords);

    //     // 2. compute segment Length
    //     let length_meters = linestring.length::<Haversine>();

    //     // 3. find src, dst elevation, compute grade, or 0.0 by default
    //     let (src_osm_id, dst_osm_id) = match (seg.node_osmids.first(), seg.node_osmids.last()) {
    //         (Some(src), Some(dst)) => Ok((src, dst)),
    //         _ => Err(OsmError::GraphConsolidationError(format!(
    //             "segment {} from way {} has an empty path",
    //             edge_id, seg.way_id
    //         ))),
    //     }?;
    //     let src_ele = graph.nodes_map.get(src_osm_id).and_then(|n| n.ele.clone());
    //     let dst_ele = graph.nodes_map.get(dst_osm_id).and_then(|n| n.ele.clone());
    //     let grade: Grade = match (src_ele, dst_ele) {
    //         (Some(src_ele_str), Some(dst_ele_str)) => {
    //             let src_ele = match src_ele_str.parse::<f64>() {
    //                 Ok(v) => Ok(v),
    //                 Err(_) if ignore_osm_parsing_errors => Ok(0.0),
    //                 Err(e) => Err(OsmError::GraphConsolidationError(format!(
    //                     "failure parsing elevation value {} for osm node {}: {}",
    //                     src_ele_str, src_osm_id, e
    //                 ))),
    //             }?;
    //             let dst_ele = match dst_ele_str.parse::<f64>() {
    //                 Ok(v) => Ok(v),
    //                 Err(_) if ignore_osm_parsing_errors => Ok(0.0),
    //                 Err(e) => Err(OsmError::GraphConsolidationError(format!(
    //                     "failure parsing elevation value {} for osm node {}: {}",
    //                     dst_ele_str, dst_osm_id, e
    //                 ))),
    //             }?;
    //             let rise = dst_ele - src_ele;
    //             let grade = rise / length_meters;
    //             Ok(Grade::new(grade))
    //         }
    //         _ => Ok(Grade::ZERO),
    //     }?;

    //     // 4. find speed/length pairs, compute maxspeed, or fallback to lookup table
    //     let maxspeed_opt = way
    //         .get_maxspeed(true)
    //         .map_err(OsmError::GraphConsolidationError);
    //     let speed = match maxspeed_opt {
    //         Ok(Some((maxspeed, speed_unit))) => {
    //             Ok(speed_unit.convert(&maxspeed, &SpeedUnit::KPH))
    //         }
    //         Ok(None) => get_fill_value(way, &maxspeeds_fill_lookup),
    //         Err(_) if ignore_osm_parsing_errors => get_fill_value(way, &maxspeeds_fill_lookup),
    //         Err(e) => Err(e),
    //     }?;

    //     // 5. create edge
    //     let edge = Edge::new(edge_id, seg.src_vertex_id, seg.dst_vertex_id, length_meters);
    //     edges.push(edge);
    //     geometries.push(geo::Geometry::LineString(linestring));
    //     grades.push(grade);
    //     maxspeeds.push(speed);
    // }
    // eprintln!();

    // if !errors.is_empty() {
    //     log::info!(
    //         "failed to build {} consolidated edges. first 5:",
    //         errors.len()
    //     );
    //     for e in errors.iter().take(5) {
    //         log::info!("{}", e);
    //     }
    // }

    // let result = CompassOsmGraphData {
    //     vertices: merged_vertices,
    //     edges,
    //     geometries,
    //     grades,
    //     maxspeeds,
    //     grade_unit: routee_compass_core::model::unit::GradeUnit::Decimal,
    //     speed_unit: routee_compass_core::model::unit::SpeedUnit::KPH,
    // };
    // Ok(result)
    todo!()
}

/// buffers the vertex geo::Points of the endpoints of the simplified graph
/// by some distance radius. returns the buffered geometries with matching
/// indices to the incoming endpoints dataset.
///
/// output geometries are in web mercator projection.
pub fn buffer_nodes(
    graph: &OsmGraph,
    radius: (Distance, DistanceUnit),
) -> Result<Vec<(OsmNodeId, Polygon<f32>)>, OsmError> {
    let (rad, rad_unit) = radius;
    let bar = Arc::new(Mutex::new(
        Bar::builder()
            .total(graph.n_connected_nodes())
            .desc(format!("node buffering ({} {})", rad.as_f64(), rad_unit))
            .build()
            .map_err(OsmError::InternalError)?,
    ));

    let result = graph
        .connected_node_data_iterator(false)
        .collect::<Result<Vec<_>, _>>()?
        .into_par_iter()
        .map(|node| {
            let point = geo::Point(Coord::from((node.x, node.y)));
            let circle_g: Geometry<f32> = point.buffer(radius).map_err(|e| {
                OsmError::GraphConsolidationError(format!(
                    "while buffering nodes for consolidation, an error occurred: {}",
                    e
                ))
            })?;
            let circle = match circle_g {
                Geometry::Polygon(polygon) => polygon,
                _ => {
                    return Err(OsmError::GraphConsolidationError(
                        "buffer of point produced non-polygonal geometry".to_string(),
                    ));
                }
            };
            if let Ok(mut b) = bar.clone().lock() {
                let _ = b.update(1);
            }
            Ok((node.osmid, circle))
        })
        .collect::<Result<Vec<_>, OsmError>>();

    result
}

fn get_fill_value(
    way: &OsmWayData,
    maxspeeds_fill_lookup: &FillValueLookup,
) -> Result<Speed, OsmError> {
    let highway_class = way
        .get_string_at_field("highway")
        .map_err(OsmError::GraphConsolidationError)?;
    let avg_speed = maxspeeds_fill_lookup.get(&highway_class);
    Ok(Speed::from(avg_speed))
}

/// with knowledge of which geometry indices contain spatially-similar nodes,
/// constructs new merged node data for the connected sub-clusters, assigning
/// the sub-cluster centroid as the new spatial coordinate.
fn consolidate_clusters(
    spatial_clusters: &[Vec<OsmNodeId>],
    graph: &mut OsmGraph,
) -> Result<Vec<OsmNodeData>, OsmError> {
    // for each spatial cluster,
    //   find sub-clusters by running a connected components search
    //   over the graph subset included in this spatial cluster

    // what keeps getting confused here is that we come up with the
    // endpoint indices somewhere between creating the SimplifiedGraph instance
    // and calling merge. we currently need to be able to bfs the simplified graph while
    // using geometry indices since the spatial clusters are collections of geometry indices.
    // perhaps they should instead be simplified graph node OSMIDs.

    log::info!(
        "consolidate clusters called with {} clusters over {} nodes",
        spatial_clusters.len(),
        spatial_clusters.iter().map(|c| c.len()).sum::<usize>()
    );

    // log::info!(
    //     "first five spatial clusters: \n{}",
    //     spatial_clusters
    //         .iter()
    //         .take(5)
    //         .map(|v| format!("{:?}", v))
    //         .join("\n")
    // );

    // let bar = Arc::new(Mutex::new(
    //     Bar::builder()
    //         .total(spatial_clusters.len())
    //         .desc("consolidate nodes")
    //         .build()
    //         .map_err(OsmError::InternalError)?,
    // ));
    // let nodes_in_merged: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    // let mut merged_results = spatial_clusters
    let outer_iter = tqdm!(
        spatial_clusters.iter(),
        total = spatial_clusters.len(),
        desc = "consolidate nodes"
    );
    for cluster in outer_iter {
        // run connected components to find the connected sub-graphs of this spatial cluster.
        // merge any discovered sub-components into a new node.
        let inner_iter = tqdm!(
            ccc(cluster, graph).into_iter(),
            desc = "find connected subgraphs within cluster"
        );
        for connected_clusters in inner_iter {
            for node_ids in connected_clusters.into_iter() {
                consolidate_nodes(node_ids, graph)?;
            }
        }
    }
    // .iter()
    // .map(|cluster| {
    //     // progress bar
    //     let b_thread = bar.clone();
    //     let mut b_lock = b_thread
    //         .lock()
    //         .map_err(|e| OsmError::GraphConsolidationError(e.to_string()))?;
    //     let _ = b_lock.update(1);

    //     // run connected components to find the connected sub-graphs of this spatial cluster.
    //     // merge any discovered sub-components into a new node.
    //     for connected_clusters in ccc(cluster, graph).into_iter() {
    //         for node_ids in connected_clusters.into_iter() {
    //             let node = create_merged_node(node_ids, &mut graph)?;
    //             graph.add_node(node)?;
    //         }
    //     }
    // ccc(cluster, graph)
    //     .into_iter()
    //     .map(|connected_clusters| {
    //         for node_ids in connected_clusters.into_iter() {
    //             let node = create_merged_node(node_ids, &mut graph);
    //         }
    //         // let merged = connected_clusters
    //         //     .into_iter()
    //         //     .map(|node_ids| create_merged_node(node_ids, &mut graph))
    //         //     .collect::<Result<Vec<_>, _>>()?;

    //         // let local_nim = nodes_in_merged.clone();
    //         // if let Ok(mut nim) = local_nim.lock() {
    //         //     *nim += merged.iter().map(|m| m.osmids.len()).sum::<usize>();
    //         // }
    //         Ok(merged)
    //     })
    //     .collect::<Result<Vec<_>, OsmError>>()
    // })
    // .collect::<Result<Vec<Vec<_>>, OsmError>>()?
    // .into_iter()
    // .flatten()
    // .flatten()
    // .collect::<Vec<_>>();

    // once again sorted for algorithmic determinism.
    // merged_results.sort_by_key(|row| row.osmids.clone());

    // log::info!(
    //     "first ten merged results: \n{}",
    //     merged_results
    //         .iter()
    //         .take(10)
    //         .map(|m| format!("{:?}", m))
    //         .join("\n")
    // );

    // log::info!("merged_results has {} entries", merged_results.len());

    // validate that we didn't drop any endpoints in the process
    // let nodes_in_merged: Vec<OsmNodeId> = merged_results
    //     .iter()
    //     .flat_map(|n| n.osmids.clone())
    //     .collect_vec();
    // let removed_simple_nodes = endpoint_index_osmid_mapping.len() - nodes_in_merged.len();
    // if removed_simple_nodes > 0 {
    //     log::info!(
    //         "merged nodes does not include {} simplified nodes which became disconnected",
    //         removed_simple_nodes
    //     );
    // }

    // Ok(merged_results)
    todo!()
}

/// helper function to merge nodes into a new node and modify the graph
/// adjacencies accordingly.
///
/// # Arguments
///
/// * `node_ids` - nodes to consolidate. these should exist in the graph
/// * `graph` - the graph to inject the consolidated node
fn consolidate_nodes(node_ids: Vec<OsmNodeId>, graph: &mut OsmGraph) -> Result<(), OsmError> {
    // arbitrarily picking an osmid from the first node to be the osmid of the new node.
    let new_node_id: OsmNodeId = *node_ids.first().ok_or_else(|| {
        OsmError::InternalError(String::from(
            "create_merged_nodes called with empty node_ids collection",
        ))
    })?;
    let old_node_id = OsmNodeId(-new_node_id.0);

    // collect the nodes to remove and create consolidated node
    let nodes = &node_ids
        .iter()
        .map(|node_id| graph.get_node_data(node_id))
        .collect::<Result<Vec<_>, OsmError>>()?;
    let node = OsmNodeData::consolidate(&new_node_id, &nodes[..])?;
    let adjacencies = node
        .consolidated_ids
        .iter()
        .map(|consolidated_node_id| {
            match graph.get_neighbors(consolidated_node_id, AdjacencyDirection::Forward) {
                None => Ok(vec![]),
                Some(neighbors) => neighbors
                    .iter()
                    .map(|dst| {
                        let way = graph.get_ways_from_od(consolidated_node_id, dst)?;
                        Ok((new_node_id, way.clone(), *dst))
                    })
                    .collect::<Result<Vec<_>, _>>(),
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect_vec();

    graph.insert_node(node)?;
    for (src, ways, dst) in adjacencies.into_iter() {
        graph.add_new_adjacency(&src, &dst, ways)?;
    }
    // graph.insert_and_attach_node(node, Some(adjacencies))?;

    // retire old nodes so that they are disconnected but their data is preserved.
    for node_id in node_ids.iter() {
        if node_id != &new_node_id {
            graph.retire_node(node_id, true)?;
        }
    }

    // find all in+out edges, replace their source node id with the new one
    update_incident_way_data(new_node_id, &node_ids, graph, AdjacencyDirection::Forward)?;
    update_incident_way_data(new_node_id, &node_ids, graph, AdjacencyDirection::Reverse)?;

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
    dir: AdjacencyDirection,
) -> Result<(), OsmError> {
    // find the ways that will be impacted by consolidation
    let remove_nodes: HashSet<&OsmNodeId> = node_ids.iter().collect();
    let mut updated_ods: HashSet<(OsmNodeId, OsmNodeId)> = HashSet::new();
    let mut update_tuples: Vec<(&OsmNodeId, &OsmNodeId, usize, OsmWayData)> = vec![];
    for src in node_ids.iter() {
        let neighbors = graph.get_neighbors(src, dir).unwrap_or_default();
        for dst in neighbors.iter() {
            updated_ods.insert((*src, *dst));
        }
    }

    for (src, dst) in updated_ods.iter() {
        let ways = graph.get_ways_from_od(src, dst)?;
        for (index, way) in ways.iter().enumerate() {
            let mut updated_way = way.clone();
            if updated_way.nodes.is_empty() {
                return Err(OsmError::InternalError(format!(
                    "way ({})-[{}]->({}) has empty node list",
                    src, updated_way.osmid, dst
                )));
            }

            // remove consolidated nodes from the Way nodelist, they are becoming a single point
            updated_way.nodes.retain(|n| !remove_nodes.contains(n));

            // insert the new node in the correct position along this way
            match dir {
                AdjacencyDirection::Forward => updated_way.nodes.insert(0, new_node_id),
                AdjacencyDirection::Reverse => updated_way.nodes.push(new_node_id),
            }

            update_tuples.push((src, dst, index, updated_way));
        }
    }

    for (src, dst, index, updated_way) in update_tuples.into_iter() {
        graph.update_way(src, dst, index, updated_way)?;
    }
    Ok(())
}

/// connected components clustering algorithm.
/// finds the full set of sub-components within the provided set of
/// geometry indices.
///
/// # Arguments
/// * `geometry_indices`             - indices into the spatial intersection vector that
///                                    will be considered for clustering
/// * `simplified`                   - the simplified graph
/// * `endpoint_index_osmid_mapping` - maps indices to Node OSMIDs
///
/// # Returns
///
/// A vector of vectors, each representing the sub-graph of the spatial cluster that is
/// connected in the simplified graph. these are Node OSMIDs so that that can be used to build a MergedNodeData
/// over a new vector of indexed [`MergedNodeData`].
fn ccc(cluster_ids: &[OsmNodeId], graph: &OsmGraph) -> Result<Vec<Vec<OsmNodeId>>, OsmError> {
    // handle trivial cases that do not require executing this algorithm
    match *cluster_ids {
        [] => return Ok(vec![]),
        [singleton] => {
            return Ok(vec![vec![singleton]]);
        }
        _ => {}
    };

    let mut clusters: Vec<Vec<OsmNodeId>> = vec![];
    let mut assigned: HashSet<OsmNodeId> = HashSet::default();

    // build the iterator over the nodes in the spatial overlay result, but instead
    // of using their geometry index, use their NodeOSMID.
    // only do a progress bar for non-trivial sizes of the geometry_ids argument
    // such as things larger than a road network intersection.
    let use_progress_bar = cluster_ids.len() > 1000;
    let cc_iter: Box<dyn Iterator<Item = &OsmNodeId>> = if use_progress_bar {
        Box::new(tqdm!(
            cluster_ids.iter(),
            total = cluster_ids.len(),
            desc = "connected components"
        ))
    } else {
        Box::new(cluster_ids.iter())
    };

    // store the NodeOsmids for quick lookup (the "valid set")
    let valid_set: HashSet<OsmNodeId> = cluster_ids.iter().cloned().collect::<HashSet<_>>();

    // as we iterate through each of the node ids in this spatial cluster,
    // we are looking to assign them to at least one sub-graph.
    for this_node_id in cc_iter {
        if !assigned.contains(this_node_id) {
            // found a label that is unassigned. begin the next cluster.
            // for each clustered geometry index, label it assigned and add it to this cluster
            let clustered_nodes = bfs_undirected(*this_node_id, graph, Some(&valid_set))?;
            let next_cluster = clustered_nodes
                .iter()
                .map(|n| {
                    assigned.insert(*n);
                    *n
                })
                .collect_vec();
            clusters.push(next_cluster);
        }
    }
    if use_progress_bar {
        eprintln!();
    }
    let out_size: usize = clusters.iter().map(|c| c.len()).sum();
    if out_size != cluster_ids.len() {
        // all nodes should be assigned to exactly one output vector.
        return Err(OsmError::GraphConsolidationError(format!(
            "ccc input size != output size ({} != {})",
            cluster_ids.len(),
            out_size
        )));
    }
    Ok(clusters)
}

// /// finds the set of indices that are part of the same geometry cluster
// /// using a breadth-first search over an undirected graph of geometry
// /// intersection relations.
// ///
// fn bfs(src: OsmNodeId, valid_set: &HashSet<OsmNodeId>, graph: &OsmGraph) -> HashSet<OsmNodeId> {
//     // initial search state. if a NodeOsmid has been visited, it is appended
//     // to the visited set.
//     // breadth-first search is modeled here with a linked list FIFO queue.
//     let mut visited: HashSet<OsmNodeId> = HashSet::new();
//     let mut frontier: LinkedList<OsmNodeId> = LinkedList::new();
//     // let mut frontier = BinaryHeap::new();
//     visited.insert(src);
//     // frontier.push(Reverse((0, src)));
//     frontier.push_back(src);

//     // while let Some(Reverse((next_depth, next_id))) = frontier.pop() {
//     while let Some(next_id) = frontier.pop_front() {
//         // add to the search tree
//         visited.insert(next_id);

//         // expand the frontier
//         let next_in = graph.in_neighbors.get(&next_id);
//         let next_out = graph.out_neighbors.get(&next_id);
//         let neighbors: Box<dyn Iterator<Item = &OsmNodeId>> = match (next_in, next_out) {
//             (None, None) => Box::new(std::iter::empty()),
//             (None, Some(b)) => Box::new(b.iter()),
//             (Some(a), None) => Box::new(a.iter()),
//             (Some(a), Some(b)) => Box::new(a.union(b)),
//         };
//         // neighbors are only reviewed here that are "valid" (all fall within the spatial cluster).
//         // they are sorted for algorithmic determinism (frontier insertion order).
//         let valid_neighbors = neighbors.filter(|n| valid_set.contains(*n)).sorted();
//         for neighbor in valid_neighbors {
//             if !visited.contains(neighbor) {
//                 // frontier.push(Reverse((next_depth + 1, *neighbor))); // min heap
//                 frontier.push_back(*neighbor); // min heap
//             }
//         }
//     }
//     visited
// }
