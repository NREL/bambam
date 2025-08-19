use bambam::app::{
    oppvec::{self, oppvec_ops},
    overlay::{self, OverlayOperation, OverlaySource},
};
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct CliArgs {
    #[command(subcommand)]
    app: App,
}

use bambam::app::wci;
use bambam::model::input_plugin::grid::extent_format::ExtentFormat;
use bambam::model::input_plugin::grid::grid_input_plugin;
use bambam::model::input_plugin::grid::grid_input_plugin_builder;
use bambam::model::input_plugin::grid::grid_type::GridType;
use bambam::model::input_plugin::population::population_source_config::PopulationSourceConfig;
use bamsoda_acs::model::AcsType;
use bamsoda_core::model::identifier::GeoidType;
use h3o::Resolution;
use serde_json::json;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

#[derive(Subcommand)]
pub enum App {
    #[command(
        name = "walk_comfort_index",
        about = "calculate the WCI of links, set to file"
    )]
    WalkComfortIndexSet {
        /// file to write WCI values to, one per line
        #[arg(long)]
        wci_file: String,
        /// input csv file with edges OSM data
        #[arg(long)]
        edges_osm: String,
        /// input csv file with vertices OSM data
        #[arg(long)]
        vertices_osm: String,
    },
    #[command(
        name = "preprocess_grid",
        about = "processs the grid before running bambam to avoid time-out errors"
    )]
    PreProcessGrid {
        /// 1 or 5 for one- or five-year ACS (American Community Survey) population estimate
        #[arg(long)]
        acs_type: AcsType,
        /// year for estimate
        #[arg(long)]
        acs_year: u64,
        /// (Optional) GeoidType for resolution
        #[arg(long)]
        acs_resolution: Option<GeoidType>,
        /// (Optional) String for comma-separated categories
        #[arg(long)]
        acs_categories: Option<String>,
        /// (Optional) String for api token
        #[arg(long)]
        api_token: Option<String>,
        /// format of the extent in ExtentFormat, Wkt
        #[arg(long)]
        extent_format: ExtentFormat,
        /// Resolution of grid, value 0-15
        #[arg(long)]
        grid_resolution: Resolution,
        /// String of desired output file location
        #[arg(long)]
        output_file: String,
        /// String of extent to examine
        #[arg(long)]
        extent: String,
    },
    #[command(
        name = "opps-long",
        about = "vectorize an opportunity dataset CSV in long format for bambam integration"
    )]
    OpportunitiesLongFormat {
        /// a vertices-compass.csv.gz file for a RouteE Compass dataset
        vertices_compass_filename: String,
        /// a CSV file containing opportunities and geometries in long format
        opportunities_filename: String,
        /// file to write resulting opportunities dataset, designed to be a tabular
        /// opportunity input to bambam.
        output_filename: String,
        /// column name containing WKT geometry. cannot be used when x|y columns are specified.
        #[arg(long)]
        x_column: Option<String>,
        /// column name containing x coordinates. cannot be used when "geometry_column" is specified.
        #[arg(long)]
        y_column: Option<String>,
        /// column name containing y coordinates. cannot be used when "geometry_column" is specified.
        #[arg(long)]
        geometry_column: Option<String>,

        /// column name containing activity category name
        #[arg(long)]
        category_column: String,

        // /// optional column name containing activity counts. if omitted, counts each row as 1 opportunity.
        #[arg(long)]
        count_column: Option<String>,
        /// mapping from column name to activity type as comma-delimited string of "col->acts" statements, where
        /// "col" is the source column name, and "acts" is a hyphen-delminited non-empty list of target activity categories.
        /// example: "CNS07->retail-jobs,CNS16->healthcare-jobs,CNS05->jobs"
        #[arg(long)]
        column_mapping: String,
        // // / comma-delimited list of categories to keep
        // #[arg(long)]
        // activity_categories: String,
    },
    #[command(
        name = "opps-wide",
        about = "vectorize an opportunity dataset CSV for bambam integration"
    )]
    OpportunitiesWideFormat {
        // source_format: SourceFormat,
        /// a vertices-compass.csv.gz file for a RouteE Compass dataset
        vertices_compass_filename: String,
        /// a CSV file containing opportunities and geometries in long format
        opportunities_filename: String,
        /// file to write resulting opportunities dataset, designed to be a tabular
        /// opportunity input to bambam.
        output_filename: String,
        /// column name containing WKT geometry. cannot be used when x|y columns are specified.
        #[arg(long)]
        x_column: Option<String>,
        /// column name containing x coordinates. cannot be used when "geometry_column" is specified.
        #[arg(long)]
        y_column: Option<String>,
        /// column name containing y coordinates. cannot be used when "geometry_column" is specified.
        #[arg(long)]
        geometry_column: Option<String>,
        /// mapping from column name to activity type as comma-delimited string of "col->acts" statements, where
        /// "col" is the source column name, and "acts" is a hyphen-delminited non-empty list of target activity categories.
        /// example: "CNS07->retail-jobs,CNS16->healthcare-jobs,CNS05->jobs"
        #[arg(long)]
        column_mapping: String,
        // /// comma-delimited list of categories to keep
        // #[arg(long)]
        // activity_categories: String,
    },
    #[command(
        name = "overlay-shapefile",
        about = "aggregate a bambam output to some other geospatial dataset via some overlay operation"
    )]
    OverlayShapefile {
        /// a CSV file containing a bambam output
        mep_matrix_filename: String,
        /// a file containing WKT geometries tagged with ids
        overlay_filename: String,
        /// file path to write the result dataset
        output_filename: String,
        /// overlay method to apply
        #[arg(long, default_value_t = OverlayOperation::Intersection)]
        how: OverlayOperation,
        /// name of the id field in the shapefile
        #[arg(long, default_value_t = String::from("GEOID"))]
        id_field: String,
    },
    #[command(
        name = "overlay-csv",
        about = "aggregate a bambam output to some other geospatial dataset via some overlay operation"
    )]
    OverlayCsv {
        /// a CSV file containing a bambam output
        mep_matrix_filename: String,
        /// a file containing WKT geometries tagged with ids
        overlay_filename: String,
        /// file path to write the result dataset
        output_filename: String,
        /// overlay method to apply
        #[arg(long, default_value_t = OverlayOperation::Intersection)]
        how: OverlayOperation,
        /// name of the id field in the shapefile
        #[arg(long, default_value_t = String::from("GEOID"))]
        id_column: String,
        /// name of the id field in the shapefile
        #[arg(long, default_value_t = String::from("geometry"))]
        geometry_column: String,
    },
}

impl App {
    pub fn run(&self) -> Result<(), String> {
        env_logger::init();
        match self {
            Self::WalkComfortIndexSet {
                wci_file,
                edges_osm,
                vertices_osm,
            } => {
                if let Err(error) = wci::process_wci(&edges_osm, &vertices_osm, &wci_file){
                    eprintln!("error! {:?}", error);
                }
                Ok(())
            }
            Self::PreProcessGrid {
                acs_type,
                acs_year,
                acs_resolution,
                acs_categories,
                api_token,
                extent_format,
                grid_resolution,
                output_file,
                extent,
            } => {
                // build acs categories from &Option<String> to Option<Vec<String>>
                let acs_categories: Option<Vec<String>> = acs_categories
                    .as_ref()
                    .map(|str| str.split(',').map(|elem| elem.trim().to_string()).collect());

                // create popconfig
                let pop_config = PopulationSourceConfig::UsCensusAcs {
                    acs_type: *acs_type,
                    acs_year: *acs_year,
                    acs_resolution: *acs_resolution,
                    acs_categories,
                    api_token: api_token.clone(),
                };

                // Using grid_resolution, build grid_type:Gridtype
                let grid_res_add = *grid_resolution;
                let grid_type = GridType::H3 {
                    resolution: grid_res_add,
                };

                // unpack the command line arguments into serde_json::Values
                let mut data: serde_json::Value = json!({
                    "extent": extent,
                    "population_source": pop_config,
                    "extent_format": extent_format,
                    "grid": grid_type,
                    "output_file": output_file
                });

                // BUILD THE PLUGIN
                let plugin = grid_input_plugin_builder::plugin_builder(&data).expect("Error");

                // PROCESS
                let _processed_plugin = grid_input_plugin::process_grid_input(
                    &mut data,
                    plugin.extent_format,
                    plugin.grid_type,
                    &plugin.population_source,
                );

                // mutable data as input to process_grid_input becomes a json array
                // these 3 lines make sure the resulting data array is json, if it is, we have an array to loop through
                let array = match data.as_array() {
                    Some(a) => a,
                    None => return Err("not an array of JSON".to_string()),
                };

                // write the resulting array to the output file location as newline-delimited JSON
                let file = File::create(output_file).map_err(|e| e.to_string())?;
                let mut writeto = BufWriter::new(file);
                for value in array {
                    let json_line = serde_json::to_string(value).map_err(|e| e.to_string())?;
                    writeln!(writeto, "{json_line}").map_err(|e| e.to_string())?;
                }
                println!("Wrote newline-delimited JSON to {output_file}");
                Ok(())
            }
            Self::OverlayShapefile {
                mep_matrix_filename,
                overlay_filename,
                output_filename,
                how,
                id_field,
            } => {
                let overlay_source = OverlaySource::Shapefile {
                    file: overlay_filename.clone(),
                    id_field: id_field.clone(),
                };
                overlay::run(mep_matrix_filename, output_filename, &overlay_source, how)
            }
            Self::OverlayCsv {
                mep_matrix_filename,
                overlay_filename,
                output_filename,
                how,
                id_column,
                geometry_column,
            } => {
                let overlay_source = OverlaySource::Csv {
                    file: overlay_filename.clone(),
                    geometry_column: geometry_column.clone(),
                    id_column: id_column.clone(),
                };
                overlay::run(mep_matrix_filename, output_filename, &overlay_source, how)
            }
            Self::OpportunitiesLongFormat {
                vertices_compass_filename,
                opportunities_filename,
                output_filename,
                geometry_column,
                x_column,
                y_column,
                category_column,
                count_column,
                column_mapping,
                // activity_categories,
            } => {
                let geometry_format = oppvec::GeometryFormat::new(
                    geometry_column.as_ref(),
                    x_column.as_ref(),
                    y_column.as_ref(),
                )?;
                let category_mapping = oppvec_ops::create_mapping(column_mapping)?;
                log::debug!(
                    "category mapping:\n{}",
                    serde_json::to_string_pretty(&category_mapping).unwrap_or_default()
                );
                let source_format = oppvec::SourceFormat::LongFormat {
                    geometry_format,
                    category_column: category_column.clone(),
                    count_column: count_column.clone(),
                    category_mapping,
                };
                oppvec::run(
                    vertices_compass_filename,
                    opportunities_filename,
                    output_filename,
                    &source_format,
                    // &cats,
                )
            }
            Self::OpportunitiesWideFormat {
                vertices_compass_filename,
                opportunities_filename,
                output_filename,
                geometry_column,
                x_column,
                y_column,
                column_mapping,
                // activity_categories,
            } => {
                let geometry_format = oppvec::GeometryFormat::new(
                    geometry_column.as_ref(),
                    x_column.as_ref(),
                    y_column.as_ref(),
                )?;
                if column_mapping.is_empty() {
                    return Err(String::from(
                        "cannot build wide-format source with empty column mapping",
                    ));
                }
                let column_mapping = oppvec_ops::create_mapping(column_mapping)?;
                log::debug!(
                    "column mapping:\n{}",
                    serde_json::to_string_pretty(&column_mapping).unwrap_or_default()
                );
                let source_format = oppvec::SourceFormat::WideFormat {
                    geometry_format,
                    column_mapping,
                };
                // let cats = activity_categories
                //     .split(",")
                //     .map(|c| c.to_owned())
                //     .collect_vec();
                oppvec::run(
                    vertices_compass_filename,
                    opportunities_filename,
                    output_filename,
                    &source_format,
                    // &cats,
                )
            }
        }
    }
}

fn main() {
    let args = CliArgs::parse();
    args.app.run().unwrap();
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bambam::app::oppvec;

    #[allow(unused)]
    #[ignore = "working test case"]
    fn test_vec() {
        let source_format = oppvec::SourceFormat::LongFormat {
            geometry_format: oppvec::GeometryFormat::XYColumns {
                x_column: String::from("longitude"),
                y_column: String::from("latitude"),
            },
            category_column: String::from("activity_type"),
            count_column: None,
            category_mapping: HashMap::from([
                (String::from("retail"), vec![String::from("retail")]),
                (String::from("services"), vec![String::from("services")]),
                (String::from("food"), vec![String::from("food")]),
                (String::from("healthcare"), vec![String::from("healthcare")]),
                (
                    String::from("entertainment"),
                    vec![String::from("entertainment")],
                ),
            ]),
        };
        let result = oppvec::run(
            &String::from("/Users/rfitzger/dev/nrel/routee/routee-compass-tomtom/data/tomtom_denver/vertices-compass.csv.gz"),
            &String::from("/Users/rfitzger/data/mep/mep3/input/opportunities/costar/2018-04-costar-mep-long.csv"),
            "",
            &source_format,
        );
        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{e}");
            }
        }
    }
}
