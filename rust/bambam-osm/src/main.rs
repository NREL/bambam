use bambam_osm::{
    config::OsmImportConfiguration,
    model::{
        osm::{graph::CompassWriter, OsmSource},
        OsmCliError,
    },
};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct OsmAppArguments {
    #[command(subcommand)]
    app: App,
}

#[derive(Subcommand)]
pub enum App {
    Pbf {
        #[arg(long, help = "path to .pbf file for import")]
        pbf_file: String,
        #[arg(long, help = "path to file containing WKT used to filter the PBF data")]
        extent_file: Option<String>,
        #[arg(long, help = "path to file with bambam-osm import parameters")]
        configuration_file: Option<String>,
        #[arg(long, help = "output path for network dataset")]
        output_directory: String,
    },
}

pub fn run(app: &App) -> Result<(), OsmCliError> {
    env_logger::init();
    match app {
        App::Pbf {
            pbf_file,
            extent_file,
            configuration_file, // network_filter,
            output_directory,
        } => {
            let conf = match configuration_file {
                None => Ok(OsmImportConfiguration::default()),
                Some(f) => {
                    log::info!("reading bambam configuration from {f}");
                    OsmImportConfiguration::try_from(f)
                }
            }?;
            let consolidation_threshold = conf.get_consolidation_threshold();
            let out_path = Path::new(output_directory);
            let pbf_config = OsmSource::Pbf {
                pbf_filepath: pbf_file.clone(),
                extent_filter_filepath: extent_file.clone(),
                network_filter: Some(conf.element_filter),
                component_filter: Some(conf.component_filter),
                truncate_by_edge: conf.truncate_by_edge,
                ignore_errors: conf.ignore_osm_parsing_errors,
                simplify: conf.simplify,
                consolidate: conf.consolidate,
                consolidation_threshold,
                parallelize: conf.parallelize,
            };
            let graph = pbf_config.import()?;
            match graph.write_compass(out_path, true) {
                Ok(_) => {
                    eprintln!("finished.");
                    Ok(())
                }
                Err(e) => {
                    log::error!("bambam-osm failed: {e}");
                    Err(e)?
                }
            }
        }
    }
}

fn main() {
    let args = OsmAppArguments::parse();
    match run(&args.app) {
        Ok(_) => {
            // if !s.is_empty() {
            //     println!("{}", s);
            // }
        }
        Err(e) => {
            println!("{e}");
            // log::error!("app failed: {}", e);
            panic!("{}", e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use bambam_osm::model::osm::graph::{OsmNodeDataSerializable, OsmWayDataSerializable};
    use routee_compass_core::util::fs::read_utils;
    use std::collections::HashMap;

    #[test]
    fn test_e2e_liechtenstein() {
        // uses a small OSM dataset to test the end-to-end data processing
        let cleanup_tmp_dir = match std::env::var("CLEANUP_TMP_DIR") {
            Ok(value) => value
                .parse::<bool>()
                .expect("CLEANUP_TMP_DIR must be 'true' or 'false'"),
            Err(_) => true, // default
        };
        let temp_directory = "src/test/tmp";
        let pbf_file = "src/test/liechtenstein-latest.osm.pbf";
        let extent_file = "src/test/schaan_liechtenstein.txt";
        let config_file = "../../configuration/bambam-osm/test_osm_import.toml";
        let conf = crate::App::Pbf {
            pbf_file: pbf_file.to_string(),
            extent_file: Some(extent_file.to_string()),
            configuration_file: Some(config_file.to_string()),
            output_directory: temp_directory.to_string(),
        };

        if let Err(e) = crate::run(&conf) {
            panic!("bambam-osm run failure during import: {e}");
        }

        // test graph connectivity
        let mut connectivity_is_ok = true;
        let ways_result: Result<Box<[OsmWayDataSerializable]>, _> =
            read_utils::from_csv(&"src/test/tmp/edges-complete.csv.gz", true, None, None);
        let nodes_result: Result<Box<[OsmNodeDataSerializable]>, _> =
            read_utils::from_csv(&"src/test/tmp/vertices-complete.csv.gz", true, None, None);
        let invariant_error_msg = match (&ways_result, &nodes_result) {
            (Ok(_), Ok(_)) => None,
            (Ok(_), Err(e)) => Some(format!("failed to read nodes file: {e}")),
            (Err(e), Ok(_)) => Some(format!("failed to read ways file: {e}")),
            (Err(e1), Err(e2)) => Some(format!(
                "failed to read both nodes and ways files: {e1} {e2}"
            )),
        };
        if let (Ok(ways), Ok(nodes)) = (ways_result, nodes_result) {
            let lookup = nodes.iter().enumerate().collect::<HashMap<_, _>>();
            for way in ways.iter() {
                if lookup.get(&way.src_vertex_id.0).is_none()
                    || lookup.get(&way.dst_vertex_id.0).is_none()
                {
                    connectivity_is_ok = false;
                }
            }
        }

        // cleanup before exits, order is important.
        if cleanup_tmp_dir {
            std::fs::remove_dir_all(temp_directory).expect("failed to remove tmp directory");
        }
        // error if we couldn't read the output files or if the graph connectivity was invalid.
        match invariant_error_msg {
            None if !connectivity_is_ok => {
                panic!("files were written but some ways had invalid src/dst vertex ids")
            }
            Some(msg) => panic!("{msg}"),
            _ => {}
        }
    }
}
