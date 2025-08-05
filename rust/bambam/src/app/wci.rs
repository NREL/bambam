// Walk Comfort Index (WCI) Calculation in Rust
// This program calculates a Walk Comfort Index (WCI) based on various road and sidewalk attributes.
// It reads data from a CSV file, processes each row to compute scores based on speed, sidewalk
// existence, cycleway presence, traffic signals, and stop signs, and writes the final scores
// to an output file.
// The WCI is a composite score that helps assess the walkability and comfort of urban environments.
// This will be used for implementation in the Bambam project.
// August 2025 EG

// To do: clean up the imports, add comments, perhaps integrate wci_calculate within impl WayInfo
use bambam_osm::model::{
    feature::highway::{self, Highway},
    osm::graph::OsmWayDataSerializable,
};
use csv;
use geo::prelude::*;
use geo::{
    self, algorithm::centroid::Centroid, BoundingRect, Coord, Line, LineString, Point, Rect,
};
use rayon::prelude::*;
use routee_compass_core::util::geo::PolygonalRTree;
use rstar::{primitives::Rectangle, PointDistance, RTree, RTreeObject, AABB};
use serde::Deserialize;
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
};

const MAX_SCORE: i32 = 9;

#[derive(Clone)]
pub struct RTreeStruct {
    geo: LineString<f32>,
    data: OsmWayDataSerializable,
}

impl RTreeObject for RTreeStruct {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        match self.geo.bounding_rect() {
            Some(bounding_box) => AABB::from_corners(
                [bounding_box.min().x, bounding_box.min().y],
                [bounding_box.max().x, bounding_box.max().y],
            ),
            None => AABB::from_corners([0.0, 0.0], [0.0, 0.0]),
        }
    }
}

impl PointDistance for RTreeStruct {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let query_point = geo::Point::new(point[0], point[1]);
        let midpoint: Point<f32> = self.geo.centroid().unwrap();
        let distance = Euclidean::distance(&geo::Euclidean, &midpoint, &query_point);
        distance * distance
    }
}

#[derive(Debug, Deserialize)]
pub struct WayInfo {
    speed_imp: Option<i32>,
    sidewalk_exists: Option<bool>,
    cycleway_exists: Option<(String, i32)>,
    traffic_signals_exists: Option<bool>,
    stops_exists: Option<bool>,
    dedicated_foot: Option<bool>,
    no_adjacent_roads: Option<bool>,
    walk_eligible: Option<bool>,
}

impl WayInfo {
    pub fn new(
        centroid: geo::Point<f32>,
        rtree: &RTree<RTreeStruct>,
        geo_data: &RTreeStruct,
    ) -> Option<WayInfo> {
        let query_pointf32 = [centroid.x(), centroid.y()];
        let query_point = geo::Point::new(centroid.x(), centroid.y());

        let mut sidewalk = match &geo_data.data.sidewalk {
            Some(string) => {
                if string == "no" || string == "none" {
                    false
                } else {
                    true
                }
            }
            _ => false,
        };

        let foot = match &geo_data.data.footway {
            Some(string) => {
                if string == "no" || string == "none" {
                    false
                } else {
                    true
                }
            }
            _ => false,
        };

        if geo_data.data.footway == Some("sidewalk".to_string()) {
            sidewalk = true;
        }

        // walk_el determines if the road is eligible for walking comfort index calculation
        // walk eligible if one is true: has sidewalk, has footway, has correct highway type, or adjacent sidewalk
        let mut walk_el: bool = false;
        let this_highway: Highway = geo_data.data.highway.clone();

        if sidewalk {
            walk_el = true;
        } else if foot {
            walk_el = true;
        } else if matches!(
            this_highway,
            Highway::Residential
                | Highway::Unclassified
                | Highway::LivingStreet
                | Highway::Service
                | Highway::Pedestrian
                | Highway::Trailhead
                | Highway::Track
                | Highway::Footway
                | Highway::Bridleway
                | Highway::Steps
                | Highway::Corridor
                | Highway::Path
                | Highway::Elevator
        ) {
            walk_el = true;
        } else {
            // check for adjacent sidewalks
            for neighbor in rtree.locate_within_distance(query_pointf32, 15.24) {
                if let Some(ref sidewalk) = neighbor.data.sidewalk {
                    if sidewalk != "no" && sidewalk != "none" {
                        walk_el = true;
                        break;
                    }
                } // could also be neighboring footway=sidewalk
                if neighbor.data.footway == Some("sidewalk".to_string()) {
                    walk_el = true;
                    break;
                }
            }
            walk_el = true;
        }

        if walk_el == false {
            // return immediately
            return None;
        }

        let mut neighbors = vec![];
        for neighbor in rtree.locate_within_distance(query_pointf32, 15.24) {
            neighbors.push(neighbor);
        }
        let mut no_adj: bool = true;
        if neighbors.len() == 0 {
            no_adj = true;
        } else {
            no_adj = false;
        }

        let mut traf_sig = true;
        if matches!(geo_data.data.highway, Highway::TrafficSignals) {
            traf_sig = true;
        } else {
            traf_sig = false;
        }

        let mut stops = true;
        if matches!(geo_data.data.highway, Highway::Stop) {
            stops = true;
        } else {
            stops = false;
        }

        let cycle = match &geo_data.data.cycleway {
            Some(string) => {
                if string == "lane" || string == "designated" || string == "track" {
                    ("dedicated", 2)
                } else if string == "crossing" || string == "shared" || string == "shared_lane" {
                    ("some_cycleway", 0)
                } else {
                    ("no_cycleway", -2)
                }
            }
            _ => {
                // neighbor weighting
                let mut weighted_cycle = 0;
                let mut total_lengths: f32 = 0.0;
                let mut cyclescores = vec![];
                for neighbor in rtree.locate_within_distance(query_pointf32, 15.24) {
                    let mut neighbor_cycle_score = 0;
                    let mut int_length = Euclidean::distance(
                        &geo::Euclidean,
                        neighbor.geo.centroid().unwrap(),
                        query_point,
                    );
                    total_lengths += int_length;
                    if let Some(ref cycleway) = neighbor.data.cycleway {
                        if cycleway == "lane" || cycleway == "designated" || cycleway == "track" {
                            neighbor_cycle_score = 2;
                        } else if cycleway == "crossing"
                            || cycleway == "shared"
                            || cycleway == "shared_lane"
                        {
                            neighbor_cycle_score = 0;
                        } else {
                            neighbor_cycle_score = -2;
                        }
                        cyclescores.push((neighbor_cycle_score, int_length));
                    }
                }
                let mut result_cycle: f32 = 0.0;
                for (neighbor_cyclescore, mut length) in &cyclescores {
                    let weight = length / total_lengths;
                    result_cycle += (*neighbor_cyclescore as f32) * weight;
                }
                if (cyclescores.len() > 0 && total_lengths != 0.0) {
                    ("from_neighbors", result_cycle as i32)
                } else {
                    ("no_cycleway", -2)
                }
            }
        };

        let speed: i32 = match geo_data.data.maxspeed.clone() {
            Some(speed) => speed.parse().unwrap(),
            None => {
                // look at neighbors, weighted average
                let mut speeds = vec![];
                let mut total_lengths: f32 = 0.0;
                for neighbor in rtree.locate_within_distance(query_pointf32, 15.24) {
                    let mut int_length = Euclidean::distance(
                        &geo::Euclidean,
                        neighbor.geo.centroid().unwrap(),
                        query_point,
                    );
                    total_lengths += int_length;
                    match &neighbor.data.maxspeed {
                        Some(neighbor_speed) => {
                            let int_neighbor_speed: i32 = neighbor_speed.parse().unwrap();
                            speeds.push((int_neighbor_speed, int_length));
                        }
                        None => continue,
                    }
                }
                let mut result_speed = 0.0;
                for (neighbor_speed, mut length) in &speeds {
                    let weight = length / total_lengths;
                    result_speed += (*neighbor_speed as f32) * weight;
                }
                if (speeds.len() > 0 && total_lengths != 0.0) {
                    result_speed as i32
                } else {
                    0
                }
            }
        };

        let way_info = WayInfo {
            speed_imp: Some(speed),
            sidewalk_exists: Some(sidewalk),
            cycleway_exists: Some((cycle.0.to_string(), cycle.1)),
            traffic_signals_exists: Some(traf_sig),
            stops_exists: Some(stops),
            dedicated_foot: Some(foot),
            no_adjacent_roads: Some(no_adj),
            walk_eligible: Some(walk_el),
        };

        Some(way_info)
    }
}

pub fn wci_calculate(way: WayInfo) -> i32 {
    if way.walk_eligible == Some(false) {
        0
    } else if way.dedicated_foot == Some(true)
        || (way.no_adjacent_roads == Some(true) && way.sidewalk_exists == Some(true))
    {
        MAX_SCORE
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
        final_score
    }
}

pub fn process_wci(input_file: &str, output_file: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(input_file).unwrap();
    let mut centroids = vec![];
    let mut rtree_data = vec![];

    for row in reader.deserialize() {
        let r: OsmWayDataSerializable = row.unwrap();
        let linestring = r.linestring.clone();
        let centroid = linestring.centroid().unwrap();
        let centroid_geo: geo::Point<f32> = geo::Point::new(centroid.x(), centroid.y());
        centroids.push(centroid_geo);
        let rtree_entry = RTreeStruct {
            geo: linestring,
            data: r,
        };
        rtree_data.push(rtree_entry);
    }

    let rtree = RTree::bulk_load(rtree_data.clone());
    let mut wci_vec = vec![];
    let result = centroids.into_iter().enumerate().map(|(idx, centroid)| {
        let way_info = WayInfo::new(centroid, &rtree, &rtree_data[idx]);
        let score = wci_calculate(way_info.unwrap());
        wci_vec.push(score);
    });

    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    for wci in result {
        writeln!(writer, "{:?}", wci)?;
    }
    Ok(())
}
