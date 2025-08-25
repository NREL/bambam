// WayAttributesForWCI struct is used to store information needed to calculate the Walking Comfort Index (wci.rs)
// Information in the struct is derived from OSM data and neighbors in the RTree
// August 2025 EG

use super::way_geometry_and_data::WayGeometryData;
use bambam_osm::model::{
    feature::highway::{self, Highway},
    osm::graph::OsmWayDataSerializable,
};
use geo::prelude::*;
use rstar::RTree;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WayAttributesForWCI {
    pub speed_imp: Option<i32>,
    pub sidewalk_exists: Option<bool>,
    pub cycleway_exists: Option<(String, i32)>,
    pub traffic_signals_exists: Option<bool>,
    pub stops_exists: Option<bool>,
    pub dedicated_foot: Option<bool>,
    pub no_adjacent_roads: Option<bool>,
    pub walk_eligible: Option<bool>,
}

impl WayAttributesForWCI {
    pub fn new(
        centroid: geo::Point<f32>,
        rtree: &RTree<WayGeometryData>,
        geo_data: &WayGeometryData,
    ) -> Option<WayAttributesForWCI> {
        let query_pointf32 = [centroid.x(), centroid.y()];
        let query_point = geo::Point::new(centroid.x(), centroid.y());

        let mut sidewalk = match &geo_data.data.sidewalk {
            Some(string) => {
                !(string == "no" || string == "none")
            }
            _ => false,
        };

        let foot = match &geo_data.data.footway {
            Some(string) => {
                !(string == "no" || string == "none")
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
            for neighbor in rtree.locate_within_distance(query_pointf32, 0.0001378) {
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

        if !walk_el {
            // return immediately
            return None;
        }

        let mut neighbors = vec![];
        for neighbor in rtree.locate_within_distance(query_pointf32, 0.0001378) {
            neighbors.push(neighbor);
        }
        let mut no_adj: bool = true;
        no_adj = (neighbors.is_empty());

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
                for neighbor in rtree.locate_within_distance(query_pointf32, 0.0001378) {
                    let mut neighbor_cycle_score = 0;
                    let origin = neighbor.geo.centroid();
                    if let Some(origin) = origin {
                        let mut int_length =
                            Euclidean::distance(&geo::Euclidean, origin, query_point);
                        total_lengths += int_length;
                        if let Some(ref cycleway) = neighbor.data.cycleway {
                            if cycleway == "lane" || cycleway == "designated" || cycleway == "track"
                            {
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
                    } else {
                        continue;
                    }
                }
                let mut result_cycle: f32 = 0.0;
                for (neighbor_cyclescore, mut length) in &cyclescores {
                    let weight = length / total_lengths;
                    result_cycle += (*neighbor_cyclescore as f32) * weight;
                }
                if (!cyclescores.is_empty() && total_lengths != 0.0) {
                    ("from_neighbors", result_cycle as i32)
                } else {
                    ("no_cycleway", -2)
                }
            }
        };

        let speed: i32 = match geo_data.data.maxspeed.clone() {
            Some(speed_str) => {
                if let Ok(parsed_speed) = speed_str.parse::<i32>() {
                    parsed_speed
                } else {
                    0
                }
            }
            None => {
                // look at neighbors, weighted average
                let mut speeds = vec![];
                let mut total_lengths: f32 = 0.0;
                for neighbor in rtree.locate_within_distance(query_pointf32, 0.0001378) {
                    if let Some(origin) = neighbor.geo.centroid() {
                        let int_length = Euclidean::distance(&geo::Euclidean, origin, query_point);
                        total_lengths += int_length;
                        if let Some(neighbor_speed_str) = &neighbor.data.maxspeed {
                            if let Ok(int_neighbor_speed) = neighbor_speed_str.parse::<i32>() {
                                speeds.push((int_neighbor_speed, int_length));
                            }
                        }
                    }
                }
                let mut result_speed = 0.0;
                for (neighbor_speed, length) in &speeds {
                    let weight = length / total_lengths;
                    result_speed += (*neighbor_speed as f32) * weight;
                }
                if !speeds.is_empty() && total_lengths != 0.0 {
                    result_speed as i32
                } else {
                    0
                }
            }
        };

        let way_info = WayAttributesForWCI {
            speed_imp: Some(speed),
            sidewalk_exists: Some(sidewalk),
            cycleway_exists: Some((cycle.0.to_string(), cycle.1)),
            traffic_signals_exists: Some(geo_data.traf_sig),
            stops_exists: Some(geo_data.stop),
            dedicated_foot: Some(foot),
            no_adjacent_roads: Some(no_adj),
            walk_eligible: Some(walk_el),
        };

        Some(way_info)
    }
}
