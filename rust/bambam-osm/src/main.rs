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
    use bambam_osm::model::osm::graph::{osm_element_filter::ElementFilter, CompassWriter};
    use bambam_osm::model::osm::OsmSource;
    use routee_compass_core::model::unit::{Distance, DistanceUnit};
    use std::collections::HashSet;
    use std::path::Path;

    #[ignore = "e2e test runner for OSM import"]
    #[allow(unused)]
    fn test_neighborhood_import() {
        env_logger::init();
        // let pbf_filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        //     .join("src")
        //     .join("resources")
        //     .join("test_neighborhood.pbf");
        // let pbf_file = pbf_filepath.as_path().to_str().unwrap();
        // let pbf_config = OsmSource::Pbf {
        //     pbf_filepath: String::from(pbf_file),
        //     network_filter: Some(ElementFilter::OsmnxAllPublic),
        //     extent_filter_filepath: None,
        //     component_filter: None,
        //     truncate_by_edge: true,
        //     simplify: true,
        //     consolidate: true,
        //     consolidation_threshold: (Distance::from(15.0), DistanceUnit::Meters),
        //     parallelize: false,
        // };
        let pbf_file = "/Users/rfitzger/data/mep/mep3/input/osm/colorado-latest.osm.pbf";
        use bambam_osm::model::feature::highway::Highway as H;
        let net_fltr = ElementFilter::HighwayTags {
            tags: HashSet::from([
                H::Footway,
                H::Cycleway,
                H::TertiaryLink,
                H::TrunkLink,
                H::Elevator,
                H::Secondary,
                H::Residential,
                H::Motorway,
                H::Trunk,
                H::PrimaryLink,
                H::Corridor,
                H::Primary,
                H::LivingStreet,
                H::Service,
                H::Steps,
                H::Track,
                H::Path,
                H::Trailhead,
                H::Pedestrian,
                H::MotorwayLink,
                H::Unclassified,
                H::Road,
                H::SecondaryLink,
                H::Tertiary,
            ]),
        };
        let pbf_config = OsmSource::Pbf {
            pbf_filepath: String::from(pbf_file),
            network_filter: Some(net_fltr),
            extent_filter_filepath: Some(String::from(
                "/Users/rfitzger/data/mep/mep3/input/extent/wkt_drawn_box_denver_city.txt",
            )),
            component_filter: None,
            truncate_by_edge: true,
            ignore_errors: true,
            simplify: false,
            consolidate: false,
            consolidation_threshold: (Distance::from(15.0), DistanceUnit::Meters),
            parallelize: false,
        };

        let graph = match pbf_config.import() {
            Ok(g) => g,
            Err(e) => panic!("graph import failed: {e}"),
        };
        match graph.write_compass(Path::new("out"), true) {
            Ok(_) => eprintln!("finished."),
            Err(e) => panic!("graph write failed: {e}"),
        }
    }
}
