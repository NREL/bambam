use super::ClusteredGeometry;
use crate::algorithm::bfs_undirected;
use crate::model::osm::graph::OsmGraph;
use crate::model::osm::graph::OsmNodeId;
use crate::model::osm::OsmError;
use geo::{BooleanOps, BoundingRect, Geometry, Intersects, Polygon, RemoveRepeatedPoints};
use geo::{Coord, MultiPolygon};
use itertools::Itertools;
use kdam::{tqdm, Bar, BarExt};
use rayon::prelude::*;
use routee_compass_core::model::unit::AsF64;
use routee_compass_core::model::unit::Distance;
use routee_compass_core::model::unit::DistanceUnit;
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, RTreeObject};
use std::collections::HashSet;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::sync::Mutex;
use wkt::ToWkt;

pub type ClusterLabel = usize;
pub type ClusteredIntersections = GeomWithData<Rectangle<(f32, f32)>, ClusteredGeometry>;

/// build an undirected graph of node geometries that intersect spatially.
/// clusters are represented simply, without any changes to their geometries and linear-time
/// intersection search for clusters. this performance hit is taken to avoid any edge cases
/// where manipulation via overlay operations might lead to representation errors.
pub fn build(
    geometries: &[(OsmNodeId, Polygon<f32>)],
) -> Result<RTree<ClusteredIntersections>, String> {
    // intersection performed via RTree.

    let mut rtree: RTree<ClusteredIntersections> = RTree::new();
    let iter = tqdm!(
        geometries.iter(),
        total = geometries.len(),
        desc = "spatial intersection"
    );

    // add each geometry to the rtree.
    for (index, polygon) in iter {
        let rect = rect_from_geometries(&[polygon])?;
        let query = GeomWithData::new(rect, ClusteredGeometry::new(*index, polygon.clone()));
        let intersecting = rtree
            .drain_in_envelope_intersecting(query.envelope())
            .sorted_by_key(|obj| obj.data.ids())
            .collect_vec();
        if intersecting.is_empty() {
            // nothing intersects with this new cluster, insert it and move on to the next row.
            rtree.insert(query);
        } else {
            // prepare to merge this geometry with any intersecting geometries by union.
            let mut new_cluster = ClusteredGeometry::new(*index, polygon.clone());

            for obj in intersecting.into_iter() {
                // it is still possible that none of the "intersecting" geometries actually
                // truly intersect since we only compared the bounding boxes at this point.
                if obj.data.intersects(polygon) {
                    // merge the intersecting data. since it was drained from the rtree, we are done.
                    new_cluster.merge_and_sort_with(&obj.data);
                } else {
                    // false alarm, this one doesn't actually intersect, put it back
                    // in the tree without changes since it was drained.
                    rtree.insert(obj);
                }
            }

            let new_rect = rect_from_geometries(&new_cluster.polygons())?;
            let new_obj = GeomWithData::new(new_rect, new_cluster);
            rtree.insert(new_obj);
        }
    }
    eprintln!();

    Ok(rtree)
}

// /// merges two geometries by Union that should be either Polygon or MultiPolygon geometries.
// fn merge_areal_geometries(a: &Geometry, b: &Geometry) -> Result<Geometry, String> {
//     let unioned: MultiPolygon = match (a, b) {
//         (Geometry::Polygon(a), Geometry::Polygon(b)) => Ok(a.union(b)),
//         (Geometry::Polygon(p), Geometry::MultiPolygon(mp)) => Ok(p.union(mp)),
//         (Geometry::MultiPolygon(mp), Geometry::Polygon(p)) => Ok(mp.union(p)),
//         (Geometry::MultiPolygon(a), Geometry::MultiPolygon(b)) => Ok(a.union(b)),
//         _ => Err(format!(
//             "invalid geometry types \n{} \n{}",
//             a.to_wkt(),
//             b.to_wkt()
//         )),
//     }?;
//     // Ok(geo::Geometry::MultiPolygon(unioned))
//     // let cleaned = unioned.union(&geo::MultiPolygon::new(vec![]));

//     let exteriors = unioned
//         .remove_repeated_points()
//         .iter()
//         .map(|p| Polygon::new(p.exterior().clone(), vec![]))
//         .collect_vec();
//     let no_holes = MultiPolygon::new(exteriors);
//     Ok(geo::Geometry::MultiPolygon(no_holes))
// }

// /// in order to simplify the graph, we have to identify nodes that are within
// /// some distance threshold of each other and then also confirm that they are
// /// connected in the graph space (for example, a bridge may not be connected to
// /// the roads beneath it).
// ///
// /// # Arguments
// /// * `indices` - a collection of entity indices that should correspond to
// ///               the indices of the undirected graph. can be the complete
// ///               collection of indices to find all components, or a subset
// ///               in order to identify the disjoint union of sub-subsets that
// ///               are connected through the undirected graph.
// /// * `undirected_graph` - all valid indices and their connections. in the
// ///                        context of OSM import, this may be the graph of
// ///                        spatial intersections or the graph of network
// ///                        connections.
// ///
// /// # Returns
// ///
// /// A vector of components, each represented as a set of indices.
// pub fn connected_components_clustering(
//     indices: &[OsmNodeId],
//     undirected_graph: &[Vec<OsmNodeId>],
// ) -> Result<Vec<HashSet<OsmNodeId>>, OsmError> {
//     // handle base cases
//     match *indices {
//         [] => return Ok(vec![]),
//         [singleton] => return Ok(vec![HashSet::from([singleton])]),
//         _ => {}
//     };
//     // connected components (undirected graph)
//     // run breadth-first searches from indices that have not yet been assigned a label.
//     // labels found in the search become part of the next cluster.
//     let mut clusters: Vec<HashSet<OsmNodeId>> = vec![];
//     let mut unassigned = indices
//         .iter()
//         .map(|idx| (*idx, true))
//         .collect::<HashMap<_, _>>();

//     let valid_set = indices.iter().collect::<HashSet<_>>();

//     let cc_iter = tqdm!(
//         indices.iter(),
//         total = unassigned.len(),
//         desc = "connected components"
//     );
//     for label in cc_iter {
//         if unassigned[label] {
//             // found a label that is unassigned. begin the next cluster.
//             let next_cluster: HashSet<usize> =
//                 bfs_undirected(*label, undirected_graph, &Some(valid_set)).map_err(|e| {
//                     OsmError::GraphConsolidationError(format!(
//                         "failure executing graph search for connected components: {}",
//                         e
//                     ))
//                 })?;

//             // for each clustered geometry index, label it assigned and add it to this cluster
//             for index in next_cluster.iter() {
//                 unassigned.entry(*index).and_modify(|val| {
//                     *val = false;
//                 });
//             }

//             clusters.push(next_cluster);
//         }
//     }
//     eprintln!();
//     Ok(clusters)
// }

// /// a succession of intersection tests, in increasing order of computational complexity,
// /// for finding geometric intersections.
// fn geometries_intersect(
//     a: (usize, &Geometry),
//     b: (usize, &Geometry),
//     // ns: &Vec<HashSet<usize>>,  // todo!: reintroduce this argument, respecting Arc<Mutex<T>>
// ) -> bool {
//     let (a_label, a_geom) = a;
//     let (b_label, b_geom) = b;

//     // simple identifier test. dismiss when row == column on the matrix.
//     if a_label == b_label {
//         log::debug!(
//             "geometries {},{} intersect? TRUE - matching label",
//             a_label,
//             b_label
//         );
//         return false;
//     }

//     // // did we already find a from b?
//     // let a_has_already_found_b = ns
//     //     .get(a_label)
//     //     .ok_or_else(|| out_of_index_err(b_label, ns))?
//     //     .contains(&b_label);
//     // if a_has_already_found_b {
//     //     return false;
//     // }

//     // first geometric test, only using bounding boxes.
//     if bbox_intersects(a_geom, b_geom) {
//         log::debug!(
//             "geometries {},{} intersect? TRUE - bboxes intersect",
//             a_label,
//             b_label
//         );
//         return true;
//     }

//     // final test is a true geometric intersection test (expensive).
//     let geometries_intersect = a_geom.intersects(b_geom);
//     if geometries_intersect {
//         log::debug!(
//             "geometries {},{} intersect? TRUE - matching label",
//             a_label,
//             b_label
//         );
//         return true;
//     } else {
//         log::debug!("geometries {},{} intersect? FALSE", a_label, b_label);
//         return false;
//     }
// }

// fn bbox_intersects(a: &Geometry, b: &Geometry) -> bool {
//     let a_box = match a.bounding_rect() {
//         Some(bbox) => bbox,
//         None => return false,
//     };
//     let b_box = match b.bounding_rect() {
//         Some(bbox) => bbox,
//         None => return false,
//     };
//     a_box.intersects(&b_box)
// }

// fn out_of_index_err(label: usize, ns: &[Vec<usize>]) -> String {
//     format!(
//         "neighbors expected to have index {} but only has {} elements",
//         label,
//         ns.len()
//     )
// }

// /// finds the set of indices that are part of the same geometry cluster
// /// using a breadth-first search over an undirected graph of geometry
// /// intersection relations.
// fn bfs(src: usize, ns: &[Vec<usize>]) -> Result<HashSet<usize>, String> {
//     let mut visited: HashSet<usize> = HashSet::new();
//     visited.insert(src);
//     let mut frontier: BinaryHeap<(usize, i32)> = BinaryHeap::new();
//     frontier.push((src, 0));
//     while let Some((next_id, next_depth)) = frontier.pop() {
//         visited.insert(next_id);
//         let next_neighbors = ns
//             .get(next_id)
//             .ok_or_else(|| out_of_index_err(next_id, ns))?;
//         for neighbor in next_neighbors.iter() {
//             if !visited.contains(neighbor) {
//                 frontier.push((*neighbor, next_depth - 1)); // max heap
//             }
//         }
//     }
//     Ok(visited)
// }

/// helper function to create a rectangular rtree envelope for a given geometry
fn rect_from_geometries(ps: &[&Polygon<f32>]) -> Result<Rectangle<(f32, f32)>, String> {
    if ps.is_empty() {
        return Err(String::from(
            "rect_from_geometries called with empty collection",
        ));
    }
    let mut mins = vec![];
    let mut maxs = vec![];
    for p in ps {
        let bbox_rect = p.bounding_rect().ok_or_else(|| {
            format!(
                "internal error: cannot get bounds of geometry: '{}'",
                p.to_wkt()
            )
        })?;
        mins.push(bbox_rect.min());
        maxs.push(bbox_rect.max());
    }
    let min_coord = mins
        .into_iter()
        .min_by_key(ordering_key)
        .ok_or_else(|| String::from("internal error: empty 'mins' collection"))?;
    let max_coord = maxs
        .into_iter()
        .max_by_key(ordering_key)
        .ok_or_else(|| String::from("internal error: empty 'maxs' collection"))?;
    let envelope = Rectangle::from_corners(min_coord.x_y(), max_coord.x_y());
    Ok(envelope)
}

/// called on WGS84 coordinates to create an ordering. since floating point
/// values have no ordering in Rust, we use scaling and conversion to i64
/// which is a feasible bijection since the max float values are +- 180.
fn ordering_key(coord: &Coord<f32>) -> (i64, i64) {
    let x = (coord.x * 100_000.0) as i64;
    let y = (coord.y * 100_000.0) as i64;
    (x, y)
}
