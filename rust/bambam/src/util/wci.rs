/* 
Walk Comfort Index (WCI) Calculation in Rust
This program calculates a Walk Comfort Index (WCI) based on various road and sidewalk attributes.
It reads data from a CSV file, processes each row to compute scores based on speed, sidewalk
existence, cycleway presence, traffic signals, and stop signs, and writes the final scores
to an output file.
The WCI is a composite score that helps assess the walkability and comfort of urban environments.
This will be used for implementation in the Bambam project. 
July 2025 EG
*/

use std::error::Error;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use rayon::prelude::*;
use bambam_osm::model::{
    osm::graph::OsmWayDataSerializable,
    feature::highway::{self, Highway},
};
use routee_compass_core::util::geo::PolygonalRTree;
use routee_compass_core::util::geo::haversine;
use csv;
use rstar::{primitives::Rectangle, RTree, RTreeObject, AABB};
use rstar::{PointDistance, Point};
use geo::Coord;
use geo::LineString;
use geo::HaversineDistance;
use geo::prelude::*;

const MAX_SCORE: i32 = 9;

#[derive(Debug, Deserialize)]
struct WayInfo {
    speed_imp: Option<i32>,
    sidewalk_exists: Option<bool>,
    cycleway_exists: Option<String>,
    traffic_signals_exists: Option<bool>,
    stops_exists: Option<bool>,
    dedicated_foot: Option<bool>,
    no_adjacent_roads: Option<bool>,
    walk_eligible: Option<bool>,
}

fn build_wayinfo(centroid: Point<f32>, rtree: &RTree<RTreeStruct>, geo_data: &RTreeStruct) -> WayInfo{
    // FIX: get sidewalk and foot from highway
    let sidewalk = match geo_data.data.highway{
        Highway::Sidewalk => {
            true
            } 
        _ => {
            false
        }
    };

    let foot = match geo_data.data.highway{
        Highway::Footway => {
            true
            } 
        _ => {
            false
        }
    };

    let mut walk_el: bool = true;
    let this_highway: Highway = geo_data.data.highway.clone();
    
    if sidewalk == true && matches!(this_highway, Highway::Residential | Highway::Unclassified | Highway::LivingStreet | Highway::Service
    | Highway::Pedestrian | Highway::Track | Highway::Footway | Highway::Bridleway | Highway::Steps | Highway::Corridor |
Highway::Path | Highway::Elevator) {
        walk_el = true;
    }
    else if sidewalk == false && foot == false{
        walk_el = true;
    }
    else{
        walk_el = false;
    }

    if walk_el == false{ // return immediately 
        let return_info = WayInfo {
        speed_imp: Some(0),
        sidewalk_exists: Some(false),
        cycleway_exists: Some("no_cycleway".to_string()),
        traffic_signals_exists: Some(false),
        stops_exists: Some(false),
        dedicated_foot: Some(false),
        no_adjacent_roads: Some(false),
        walk_eligible: Some(false),
        };

        return return_info;
    }

    let query_point = [centroid.x, centroid.y];

    let mut neighbors = vec![];
    for neighbor in rtree.locate_within_distance(query_point, 15.24){
        neighbors.push(neighbor);
    }
    let mut no_adj: bool = true;
    if neighbors.len() == 0{
        no_adj = true;
    }
    else{
        no_adj = false;
    }

    // get traffic signals from highway
    let traf_sig = match geo_data.data.highway{
        Highway::TrafficSignals => {
            true
        }
        _ => {
            let mut neighbor_traf_sig = false;
            for neighbor in rtree.locate_within_distance(query_point, 15.24){
                if matches!(neighbor.data.highway, Highway::TrafficSignals) {
                    neighbor_traf_sig = true; // found one neighbor with traf_sig
                    break;
                }
            }
            neighbor_traf_sig
        }
    };

    let stops = match geo_data.data.highway{
        Highway::Stop => {
            true
        }
        _ => {
            let mut neighbor_stops = false;
            for neighbor in rtree.locate_within_distance(query_point, 15.24){
                if matches!(neighbor.data.highway, Highway::Stop){
                    neighbor_stops = true; // found one neighbor with traf_sig
                    break;
                }
            }
            neighbor_stops
        }
    };

    // FIX CYCLEWAYS!
    let cycle = match geo_data.data.highway{
        Highway::Cycleway =>{
            "designated"
        }
        _ => {
            "get cycleway from neighbors"
        }
    };

   let speed: i32 = match geo_data.data.maxspeed.clone(){
        Some(speed) => speed.parse().unwrap(),
        None => {
            // look at neighbors, weighted average
            let mut speeds = vec![];
            let mut total_lengths = 0 as f64;
            for neighbor in rtree.locate_within_distance(query_point, 15.24){
                let mut int_length = Haversine.length(&neighbor.geo);
                total_lengths += int_length;
                match &neighbor.data.maxspeed{
                    Some(neighbor_speed) => {
                        let int_neighbor_speed: i32 = neighbor_speed.parse().unwrap();
                        speeds.push((int_neighbor_speed, int_length));
                    },
                    None => continue,
                }
            }
            let mut result_speed = 0.0;
            for (neighbor_speed, mut length) in &speeds{
                let weight = length/total_lengths;
                result_speed += (*neighbor_speed as f64) * weight;
            }
            if (speeds.len() > 0 && total_lengths != 0.0){
                result_speed as i32
            }
            else {
                0  
            }
        },
    };

    let way_info = WayInfo {
        speed_imp: Some(speed),
        sidewalk_exists: Some(sidewalk),
        cycleway_exists: Some(cycle.to_string()),
        traffic_signals_exists: Some(traf_sig),
        stops_exists: Some(stops),
        dedicated_foot: Some(foot),
        no_adjacent_roads: Some(no_adj),
        walk_eligible: Some(walk_el),
    };

    way_info
}

fn wci_calculate(way: WayInfo)-> i32{
    // residential, path, and track roads get neutral score ??
    if way.walk_eligible == Some(false){
        0
    }
    else if way.dedicated_foot == Some(true) || (way.no_adjacent_roads == Some(true) && way.sidewalk_exists == Some(true)) {
        MAX_SCORE
    }
    else {
        /// Speed: 0-25 mph: 2, 25-30 mph: 1, 30-40 mph: 0, 40-45 mph: -1, 45+ mph: -2
        fn speed_score(way: &WayInfo) -> i32 {
            match way.speed_imp {
                Some(speed) => {
                    let mph = (speed as f64 / 1.61).round();
                    if mph <= 25.0 {
                        2
                    }
                    else if mph > 25.0 && mph <= 30.0 {
                        1
                    }
                    else if mph > 30.0 && mph <= 40.0 {
                        0
                    }
                    else if mph > 40.0 && mph <= 45.0 {
                        -1
                    }
                    else {
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

        /// Cycleway: +2 if dedicated, 0 if some, -2 if none
        fn cycleway_score(way: &WayInfo) -> i32 {
            match way.cycleway_exists.as_deref() {
                Some(cycle_score) => {
                    if cycle_score == "dedicated" {
                        2
                    } else if cycle_score == "some_cycleway" {
                        0
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
        let final_score = speed_score(&way) + sidewalk_score(&way) + cycleway_score(&way) + signal_or_stop_score(&way);
        final_score
    }
}

#[derive(Clone)]
struct RTreeStruct {
    geo: LineString<f64>,
    data: OsmWayDataSerializable,
}

// START OF COPILOT
// Implement RTreeObject for RTreeStruct

impl RTreeObject for RTreeStruct {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        // Compute the bounding box of the LineString
        let coords = self.geo.points_iter().map(|p| [p.x(), p.y()]);
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);

        for [x, y] in coords {
            if x < min_x { min_x = x; }
            if y < min_y { min_y = y; }
            if x > max_x { max_x = x; }
            if y > max_y { max_y = y; }
        }

        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

// Optionally implement PointDistance for neighbor queries
impl PointDistance for RTreeStruct {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        // Use the minimum squared distance from the point to the linestring
        self.geo.points_iter()
            .map(|p| {
                let dx = p.x() - point[0];
                let dy = p.y() - point[1];
                dx * dx + dy * dy
            })
            .fold(f64::INFINITY, f64::min)
    }
}

// END OF COPILOT

fn process_wci(input_file: &str, output_file: &str)-> Result<(), Box<dyn Error>> {
  let reader = csv::Reader::from_path(input_file).unwrap();
  let mut centroids = vec![];
  let mut rtree_data = vec![];

  for row in reader.deserialize() {
    let r: OsmWayDataSerializable = row.unwrap();
    let linestring = r.linestring.clone();
    let centroid = linestring.centroid().unwrap();
    centroids.push(centroid);
    let rtree_entry = RTreeStruct {
        geo: linestring,
        data: row.clone(),
    };
    rtree_data.push(rtree_entry);
  }

  let rtree = RTree::bulk_load(rtree_data);
  let result = centroids.into_iter().enumerate().map(|(idx, centroid)| {
    let way_info = build_wayinfo(centroid, &rtree, &rtree_data[idx]);
    wci_calculate(way_info); 
  }).collect::<Result<Vec<_>, _>>().unwrap();

  let file = File::create(output_file)?;
  let mut writer = BufWriter::new(file);

  for way in result{
    writeln!(writer, "{}", way)?;
}
Ok(())

}
 
