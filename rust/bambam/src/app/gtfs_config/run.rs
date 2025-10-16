use std::{
    fs::DirEntry,
    path::{Path, PathBuf},
};

use config::Config;
use itertools::Itertools;
use regex::Regex;
use routee_compass::app::compass::{CompassAppConfig, SearchConfig};
use routee_compass_core::{
    config::OneOrMany,
    model::{
        network::{EdgeListConfig, EdgeListId},
        traversal::default::distance::DistanceTraversalConfig,
        unit::DistanceUnit,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;

use crate::{
    app::gtfs_config::gtfs_config_error::GtfsConfigError,
    model::{
        frontier::{
            multimodal::{MultimodalFrontierConfig, MultimodalFrontierConstraintConfig},
            time_limit::{TimeLimitConfig, TimeLimitFrontierConfig},
        },
        traversal::{
            multimodal::MultimodalTraversalConfig,
            transit::{ScheduleLoadingPolicy, TransitTraversalConfig},
        },
    },
};

pub const METADATA_FILENAME_REGEX: &str = r#"edges-gtfs-metadata-(\d+).json"#;

pub fn edges_filename(edge_list_id: EdgeListId) -> String {
    format!("edges-compass-{edge_list_id}.csv.gz")
}

pub fn schedules_filename(edge_list_id: EdgeListId) -> String {
    format!("edges-schedules-{edge_list_id}.csv.gz")
}

pub fn metadata_filename(edge_list_id: EdgeListId) -> String {
    format!("edges-gtfs-metadata-{edge_list_id}.json")
}

/// executes a run of the GTFS configuration application.
///
/// the algorithm here can be seen as the following steps:
///   1. read in some configuration file that doesn't have transit added
///   2. copy some data from this config to duplicate across transit additions
///   3. look for all gtfs metadata JSON files in a directory
///   4. for each metadata file, look for the other expected files associated with the same edge list
///   5. for each edge list bundle, inject graph, mapping, and search configuration
///   6. re-write as a TOML file to the file system
pub fn run(
    directory: &str,
    base_config_filepath: &str,
    base_config_relative_path: Option<&str>,
) -> Result<(), GtfsConfigError> {
    let base_str =
        std::fs::read_to_string(base_config_filepath).map_err(|e| GtfsConfigError::ReadError {
            filepath: base_config_filepath.to_string(),
            error: e.to_string(),
        })?;

    // we will load and modify the base TOML configuration file. in particular,
    // we are modifying the `[[graph.edge_list]]` and `[[search]]` sections.
    let mut compass_conf: CompassAppConfig =
        toml::from_str(base_str.as_str()).map_err(|e| GtfsConfigError::ReadError {
            filepath: base_config_filepath.to_string(),
            error: e.to_string(),
        })?;

    // temporary collections to modify when updating the base config
    let mut conf_graph_edge_lists: Vec<EdgeListConfig> =
        compass_conf.graph.edge_list.iter().cloned().collect_vec();
    let mut conf_search: Vec<SearchConfig> = compass_conf.search.iter().cloned().collect_vec();

    // used to deal with any offset value between base edge list ids and GTFS edge list ids
    let start_edge_list_id = conf_graph_edge_lists.len();

    // finds the travel modes that are already present in the config's edge lists.
    // ensure "transit" is one of the options.
    let mut available_modes = get_available_modes(&compass_conf)?;
    let transit_mode = "transit".to_string();
    if !available_modes.contains(&transit_mode) {
        available_modes.push(transit_mode);
    }

    // grab configuration arguments to copy into each GTFS frontier model configuration
    let (mmfc, tlfc) = get_frontier_model_arguments(&compass_conf)?;
    let time_limit = tlfc.time_limit.clone();
    let constraints = mmfc.constraints.clone();
    let max_trip_legs = mmfc.max_trip_legs as usize;

    let read_dir = std::fs::read_dir(directory).map_err(|e| GtfsConfigError::ReadError {
        filepath: directory.to_string(),
        error: e.to_string(),
    })?;
    let metadata_file_pattern = Regex::new(METADATA_FILENAME_REGEX).map_err(|e| {
        GtfsConfigError::InternalError(format!("failure building metadata filename regex: {e}"))
    })?;
    let metadata_files: Vec<DirEntry> = read_dir
        .filter(|entry| entry_matches_pattern(entry, &metadata_file_pattern))
        .try_collect()
        .map_err(|e| GtfsConfigError::ReadError {
            filepath: directory.to_string(),
            error: e.to_string(),
        })?;

    // confirm all files are found related to an edge list and create a record for each edge list entry
    let mut entries: Vec<GtfsEdgeListEntry> = metadata_files
        .into_iter()
        .map(|metadata_file| {
            let edge_list_id = get_edge_list_id(&metadata_file, &metadata_file_pattern)?;
            GtfsEdgeListEntry::new(
                edge_list_id,
                directory,
                base_config_relative_path.unwrap_or_default(),
            )
        })
        .try_collect()?;
    entries.sort_by_cached_key(|e| e.edge_list_id);

    let Some(first_gtfs_edge_list_id) = entries.first().map(|e| e.edge_list_id) else {
        return Err(GtfsConfigError::RunError(format!(
            "no metadata files found in directory {}",
            directory
        )));
    };
    let edge_list_id_offset = EdgeListId(start_edge_list_id - first_gtfs_edge_list_id.0);

    for entry in entries.into_iter() {
        //   0. fix the edge list id, if needed.
        // this allows the source config + the GTFS import to have different ideas of what
        // index the GTFS edge lists should begin at
        let edge_list_id_fix = EdgeListId(entry.edge_list_id.0 + edge_list_id_offset.0);
        let index = edge_list_id_fix.0;

        // update [[graph.edge_list]]
        //   1. step into [graph] to append the edge list file
        let edge_list_config = EdgeListConfig {
            input_file: entry.edges_input_file.to_string_lossy().to_string(),
        };
        conf_graph_edge_lists.push(edge_list_config);

        //   2. step into [search] to append traversal + frontier model configurations
        let edges_schedules_path = entry.schedules_input_file.to_string_lossy().to_string();
        let edges_metadata_path = entry.metadata_input_file.to_string_lossy().to_string();
        let available_route_ids = get_metadata_vec(&entry.metadata, "route_ids")?;
        let tm_conf = gtfs_traversal_model_config(
            &edges_schedules_path,
            &edges_metadata_path,
            &available_modes,
            &available_route_ids,
            max_trip_legs,
        );
        let fm_conf = gtfs_frontier_model_config(
            &constraints,
            &time_limit,
            &available_modes,
            &available_route_ids,
            max_trip_legs,
        );
        conf_search.push(SearchConfig {
            traversal: tm_conf,
            frontier: fm_conf,
        });
    }

    // update base configuration and write to output file
    compass_conf.graph.edge_list = OneOrMany::Many(conf_graph_edge_lists);
    compass_conf.search = OneOrMany::Many(conf_search);

    let result_conf = toml::to_string_pretty(&compass_conf).map_err(|e| {
        GtfsConfigError::RunError(format!(
            "failed to convert temporary configuration back to TOML string: {e}"
        ))
    })?;

    let conf_dir = Path::new(&base_config_filepath).parent().ok_or_else(|| {
        GtfsConfigError::RunError(
            "base_config_filepath argument is invalid, has no 'parent'.".to_string(),
        )
    })?;
    let out_filepath = conf_dir.join("gtfs-config.toml");
    std::fs::write(&out_filepath, &result_conf).map_err(|e| {
        GtfsConfigError::RunError(format!(
            "failure writing to {}: {e}",
            out_filepath.to_string_lossy()
        ))
    })?;

    Ok(())
}

/// grabs frontier configuration to copy to GTFS edge lists. assumes that, if there exist
/// one copy of MultimodalFrontierConfig and TimeLimitFrontierConfig, they are the same
/// across all edge lists.
pub fn get_frontier_model_arguments(
    base_conf: &CompassAppConfig,
) -> Result<(MultimodalFrontierConfig, TimeLimitFrontierConfig), GtfsConfigError> {
    if let Some((edge_list_id, search)) = base_conf.search.iter().enumerate().next() {
        let models = search.frontier.get("models").ok_or_else(|| GtfsConfigError::RunError(format!("key 'models' missing from traversal model configuration in edge list {edge_list_id}")))?;
        let models_vec = models.as_array().ok_or_else(|| {
            GtfsConfigError::RunError(format!(
                "traversal model key 'models' in edge list {edge_list_id} is not an array"
            ))
        })?;
        let mmfc: MultimodalFrontierConfig = find_expected_config(
            models_vec,
            EdgeListId(edge_list_id),
            "multimodal",
        )
        .map_err(|e| {
            GtfsConfigError::RunError(format!("while getting frontier model arguments, {e}"))
        })?;
        let tlfc: TimeLimitFrontierConfig = find_expected_config(
            models_vec,
            EdgeListId(edge_list_id),
            "time_limit",
        )
        .map_err(|e| {
            GtfsConfigError::RunError(format!("while getting frontier model arguments, {e}"))
        })?;

        return Ok((mmfc, tlfc));
    }
    Err(GtfsConfigError::RunError(String::from(
        "no frontier model found in configuration with multimodal arguments",
    )))
}

/// helper function for finding a deserializable configuration within a list of JSON values.
pub fn find_expected_config<T>(
    models: &[serde_json::Value],
    edge_list_id: EdgeListId,
    expected_name: &str,
) -> Result<T, GtfsConfigError>
where
    T: DeserializeOwned,
{
    let model_conf = models
        .iter()
        .find(|c| {
            if let Some(t_val) = c.get("type") {
                t_val.as_str() == Some(expected_name)
            } else {
                false
            }
        })
        .ok_or_else(|| {
            GtfsConfigError::RunError(format!(
                "edge list {edge_list_id} has no '{expected_name}' model"
            ))
        })?;
    let result: T = serde_json::from_value(model_conf.clone()).map_err(|e| {
        GtfsConfigError::RunError(format!(
            "failed to parse '{expected_name}' model config for edge list {edge_list_id}: {e}. JSON:\n{}",
            serde_json::to_string_pretty(model_conf).unwrap_or_default()
        ))
    })?;
    Ok(result)
}

/// finds what modes are already available via other edge lists in the config.
/// assumes that each edge list has a "multimodal" TraversalModel type.
/// enforces that the mode list matches the listing in the label model.
pub fn get_available_modes(base_conf: &CompassAppConfig) -> Result<Vec<String>, GtfsConfigError> {
    let lm_modes: Vec<String> = base_conf
        .label
        .get("modes")
        .ok_or_else(|| {
            GtfsConfigError::RunError("label model does not have a 'modes' key".to_string())
        })?
        .as_array()
        .ok_or_else(|| {
            GtfsConfigError::RunError(
                "label model 'modes' key does not have an array value".to_string(),
            )
        })?
        .iter()
        .enumerate()
        .map(|(idx, v)| {
            let v_str = v.as_str().ok_or_else(|| {
                GtfsConfigError::RunError(format!(
                    "label model '.modes[{idx}]' value is not a string"
                ))
            })?;
            Ok(v_str.to_string())
        })
        .try_collect()?;

    // let tm_modes: Vec<String> = base_conf.search.iter().enumerate().map(|(edge_list_id, search)| {
    //     let models = search.traversal.get("models").ok_or_else(|| GtfsConfigError::RunError(format!("key 'models' missing from traversal model configuration in edge list {edge_list_id}")))?;
    //     let models_vec = models.as_array().ok_or_else(|| GtfsConfigError::RunError(format!("traversal model key 'models' in edge list {edge_list_id} is not an array")))?;
    //     let multimodal_config = models_vec.iter().find(|c| {
    //         if let Some(t_val) = c.get("type") {
    //             t_val.as_str() == Some("multimodal")
    //         } else {
    //             false
    //         }
    //     })
    //         .ok_or_else(|| GtfsConfigError::RunError(format!("edge list {edge_list_id} has no multimodal traversal model")))?;
    //     let this_mode_value = multimodal_config.get("this_mode")
    //         .ok_or_else(|| GtfsConfigError::RunError(format!("'this_mode' key missing from multimodal traversal model in edge list {edge_list_id}")))?;
    //     let this_mode_str = this_mode_value.as_str()
    //         .ok_or_else(|| GtfsConfigError::RunError(format!("unable to read 'this_mode': {this_mode_value:?} as a string value for multimodal traversal model in edge list {edge_list_id}")))?;
    //     Ok(this_mode_str.to_string())
    // })
    // .try_collect()?;

    // if !(&lm_modes == &tm_modes) {
    //     Err(GtfsConfigError::RunError(format!(
    //         "label model modes do not match traversal model modes: \n{lm_modes:?}\n{tm_modes:?}"
    //     )))
    // } else {
    //     Ok(tm_modes)
    // }
    Ok(lm_modes)
}

/// get a vector of strings from the metadata object by some key.
pub fn get_metadata_vec(
    metadata: &serde_json::Value,
    key: &str,
) -> Result<Vec<String>, GtfsConfigError> {
    let vec_of_values = metadata
        .get(key)
        .ok_or_else(|| GtfsConfigError::RunError(format!("metadata missing '{key}' key")))?;
    let vec_of_strings: Vec<String> =
        serde_json::from_value(vec_of_values.clone()).map_err(|e| {
            GtfsConfigError::RunError(format!("metadata '{key}' is not an array of string: {e}"))
        })?;
    Ok(vec_of_strings)
}

/// generates the JSON fields expected for a transit traversal model
pub fn gtfs_traversal_model_config(
    edges_schedules: &str,
    edges_metadata: &str,
    available_modes: &[String],
    available_route_ids: &[String],
    max_trip_legs: usize,
) -> serde_json::Value {
    json![{
        "type": "combined",
        "models": [
            DistanceTraversalConfig { distance_unit: Some(DistanceUnit::Miles) },
            TransitTraversalConfig {
                edges_schedules_input_file: edges_schedules.to_string(),
                gtfs_metadata_input_file: edges_metadata.to_string(),
                schedule_loading_policy: ScheduleLoadingPolicy::All
            },
            MultimodalTraversalConfig {
                this_mode: "transit".to_string(),
                available_modes: available_modes.to_vec(),
                available_route_ids: available_route_ids.to_vec(),
                max_trip_legs: max_trip_legs as u64,
                use_route_ids: Some(true)
            }
        ]
    }]
}

/// generates the JSON fields expected for a transit frontier model
pub fn gtfs_frontier_model_config(
    constraints: &[MultimodalFrontierConstraintConfig],
    time_limit: &TimeLimitConfig,
    available_modes: &[String],
    available_route_ids: &[String],
    max_trip_legs: usize,
) -> serde_json::Value {
    json![{
        "type": "combined",
        "models": [
            TimeLimitFrontierConfig {
                time_limit: time_limit.clone(),
            },
            MultimodalFrontierConfig {
                mode: "transit".to_string(),
                constraints: constraints.to_vec(),
                available_modes: available_modes.to_vec(),
                available_route_ids: available_route_ids.to_vec(),
                use_route_ids: true,
                max_trip_legs: max_trip_legs as u64
            }
        ]
    }]
}

pub struct GtfsEdgeListEntry {
    pub edge_list_id: EdgeListId,
    pub edges_input_file: PathBuf,
    pub schedules_input_file: PathBuf,
    pub metadata_input_file: PathBuf,
    pub metadata: serde_json::Value,
}

impl GtfsEdgeListEntry {
    pub fn new(
        edge_list_id: EdgeListId,
        gtfs_edge_list_directory: &str,
        relative_path_to_gtfs_edge_list_directory: &str,
    ) -> Result<GtfsEdgeListEntry, GtfsConfigError> {
        let path =
            Path::new(relative_path_to_gtfs_edge_list_directory).join(gtfs_edge_list_directory);
        let edges_filename = edges_filename(edge_list_id);
        let edges_filepath = path.join(edges_filename);
        let schedules_filename = schedules_filename(edge_list_id);
        let schedules_filepath = path.join(schedules_filename);
        let metadata_filename = metadata_filename(edge_list_id);
        let metadata_filepath = path.join(metadata_filename);
        if !&edges_filepath.is_file() {
            Err(GtfsConfigError::ReadError {
                filepath: edges_filepath.to_string_lossy().to_string(),
                error: "file not found".to_string(),
            })
        } else if !&schedules_filepath.is_file() {
            Err(GtfsConfigError::ReadError {
                filepath: edges_filepath.to_string_lossy().to_string(),
                error: "file not found".to_string(),
            })
        } else if !&metadata_filepath.is_file() {
            Err(GtfsConfigError::ReadError {
                filepath: edges_filepath.to_string_lossy().to_string(),
                error: "file not found".to_string(),
            })
        } else {
            let metadata_string = std::fs::read_to_string(&metadata_filepath).map_err(|e| {
                GtfsConfigError::ReadError {
                    filepath: metadata_filepath.to_string_lossy().to_string(),
                    error: e.to_string(),
                }
            })?;
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_string).map_err(|e| GtfsConfigError::ReadError {
                    filepath: metadata_filepath.to_string_lossy().to_string(),
                    error: e.to_string(),
                })?;
            let entry = GtfsEdgeListEntry {
                edge_list_id,
                edges_input_file: edges_filepath,
                schedules_input_file: schedules_filepath,
                metadata_input_file: metadata_filepath,
                metadata,
            };
            Ok(entry)
        }
    }
}

/// helper function to handle
///   1. if the entry is Ok(_), test if it's filename matches the pattern
///   2. if the entry is Err(_), return true (keep the error to fail at end of combinator)
fn entry_matches_pattern(entry: &Result<DirEntry, std::io::Error>, pat: &Regex) -> bool {
    entry
        .as_ref()
        .map(|e| {
            let filename_os = e.file_name();
            let filename = filename_os.to_string_lossy();
            pat.is_match(&filename)
        })
        .unwrap_or(true)
}

/// helper function to extract the EdgeListId enumerated in a metadata filename
fn get_edge_list_id(entry: &DirEntry, pat: &Regex) -> Result<EdgeListId, GtfsConfigError> {
    let filename_os = entry.file_name();
    let filename = filename_os.to_string_lossy();
    let pat_match = pat
        .captures(&filename)
        .map(|g| g.get(1)) // capture group 0 is the entire match, group 1 is just the edge list id
        .flatten()
        .ok_or_else(|| {
            GtfsConfigError::InternalError(format!(
                "while extracting EdgeListId, file {filename} does not match pattern"
            ))
        })?;
    let edge_list_id = pat_match.as_str().parse::<usize>().map_err(|e| {
        GtfsConfigError::InternalError(format!(
            "while extracting EdgeListId, value {} was not a valid usize",
            pat_match.as_str()
        ))
    })?;
    Ok(EdgeListId(edge_list_id))
}
