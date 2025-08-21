// Walk Comfort Index (WCI) Calculation in Rust
// Input: OSM data with attributes, Output: file with WCI scores for each way, one score per line
// Utilizes self-designed wayinfostruct and osminfostruct for data handling
// August 2025 EG

use super::osminfostruct::OSMInfo;
use super::wayinfostruct::WayInfo;
use bambam_osm::model::osm::graph::OsmNodeDataSerializable;
use bambam_osm::model::{
    feature::highway::{self, Highway},
    osm::graph::OsmWayDataSerializable,
};
//use bamsoda_core::model::identifier::fips::County;
use csv;
use geo::prelude::*;
use geo::{
    self, algorithm::centroid::Centroid, BoundingRect, Coord, Line, LineString, Point, Rect,
};
use rayon::prelude::*;
use routee_compass_core::{model::network::VertexId, util::geo::PolygonalRTree};
use rstar::{primitives::Rectangle, PointDistance, RTree, RTreeObject, AABB};
use serde::Deserialize;
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
};

const MAX_SCORE: i32 = 9;

// Calculate the Walk Comfort Index (WCI) for a given way
pub fn wci_calculate(way: WayInfo) -> Option<i32> {
    if way.walk_eligible == Some(false) {
        None
    } else if way.dedicated_foot == Some(true)
        || (way.no_adjacent_roads == Some(true) && way.sidewalk_exists == Some(true))
    {
        Some(MAX_SCORE)
    } else {
        /// Speed: 0-25 mph: 2, 25-30 mph: 1, 30-40 mph: 0, 40-45 mph: -1, 45+ mph: -2
        fn speed_score(way: &WayInfo) -> i32 {
            match way.speed_imp {
                Some(speed) => {
                    let mph = (speed as f64 / 1.61).round();
                    if mph <= 25.0 {
                        2
                    } else if mph > 25.0 && mph <= 30.0 {
                        1
                    } else if mph > 30.0 && mph <= 40.0 {
                        0
                    } else if mph > 40.0 && mph <= 45.0 {
                        -1
                    } else {
                        -2
                    }
                }
                None => -2,
            }
        }

        /// Sidewalk: +2 if present, -2 if not
        fn sidewalk_score(way: &WayInfo) -> i32 {
            match way.sidewalk_exists {
                Some(value) => {
                    if value == true {
                        2
                    } else {
                        -2
                    }
                }
                None => -2,
            }
        }

        /// Cycleway: +2 if dedicated, 0 if some, -2 if none, or weighted from neihgbors
        fn cycleway_score(way: &WayInfo) -> i32 {
            match way.cycleway_exists.as_ref() {
                Some(cycle_score) => {
                    if cycle_score.0 == "dedicated" {
                        2
                    } else if cycle_score.0 == "some_cycleway" {
                        0
                    } else if cycle_score.0 == "from_neighbors" {
                        cycle_score.1 //check this works
                    } else {
                        -2
                    }
                }
                None => -2,
            }
        }

        /// Traffic Signals: +2 if traffic signals exists, 1 if stops exist, 0 if neither
        fn signal_or_stop_score(way: &WayInfo) -> i32 {
            if way.traffic_signals_exists == Some(true) {
                2
            } else if way.stops_exists == Some(true) {
                1
            } else {
                0
            }
        }

        /// Final Score: Speed + Sidewalk + Signal + Stop + Cycle
        let final_score = speed_score(&way)
            + sidewalk_score(&way)
            + cycleway_score(&way)
            + signal_or_stop_score(&way);
        Some(final_score)
    }
}

// Process the WCI score from the OSM data file
pub fn process_wci(
    edges_file: &str,
    vertices_file: &str,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    let mut vertices_reader = csv::Reader::from_path(vertices_file)?;
    let nodes: Vec<OsmNodeDataSerializable> = vertices_reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()?;

    let mut edges_reader = csv::Reader::from_path(edges_file)?;
    let mut centroids = vec![];
    let mut rtree_data = vec![];

    let mut count = 0;
    for row in edges_reader.deserialize() {
        count += 1;
        match row {
            Ok(osm_data) => {
                println!("Reading row: {}", count);
                let r: OsmWayDataSerializable = osm_data;
                let linestring = r.linestring.clone();
                let src_node = match nodes.get(r.src_vertex_id.0) {
                    Some(node) => node,
                    None => continue, // If source node is not found, skip this row
                };
                let has_stop = src_node
                    .clone()
                    .highway
                    .as_ref()
                    .map_or(false, |h| h.contains("stop"));
                let has_traf_sig = src_node
                    .clone()
                    .highway
                    .as_ref()
                    .map_or(false, |h| h.contains("traffic_signals"));
                if let Some(centroid) = linestring.centroid() {
                    let centroid_geo: geo::Point<f32> = geo::Point::new(centroid.x(), centroid.y());
                    centroids.push(centroid_geo);
                    let rtree_entry = OSMInfo {
                        geo: linestring,
                        data: r,
                        stop: has_stop,
                        traf_sig: has_traf_sig,
                    };
                    rtree_data.push(rtree_entry);
                }
            }
            Err(err) => {
                eprint!("Error reading row: {}", err);
            }
        }
    }

    let rtree = RTree::bulk_load(rtree_data.clone());
    // parallelized
    let wci_vec: Vec<i32> = centroids
        .into_par_iter()
        .enumerate()
        .filter_map(|(idx, centroid)| {
            WayInfo::new(centroid, &rtree, &rtree_data[idx])
            .and_then(|w: WayInfo| wci_calculate(w))
        })
        .collect();
    println!("wci_vec is {:?}", wci_vec);

    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "wci")?;

    for wci in wci_vec {
        writeln!(writer, "{:?}", wci)?;
    }

    Ok(())
}
