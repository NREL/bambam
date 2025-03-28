use bambam::app::{
    oppvec::{self, SourceFormat},
    overlay::{self, OverlayOperation},
};
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct CliArgs {
    #[command(subcommand)]
    app: App,
}

#[derive(Subcommand)]
pub enum App {
    #[command(
        name = "opp-vec",
        about = "vectorize an opportunity dataset CSV for bambam integration"
    )]
    OpportunityVectorization {
        #[command(subcommand)]
        source_format: SourceFormat,
        /// a vertices-compass.csv.gz file for a RouteE Compass dataset
        vertices_compass_filename: String,
        /// a CSV file containing opportunities and geometries in long format
        opportunities_filename: String,
        /// file to write resulting opportunities dataset, designed to be a tabular
        /// opportunity input to bambam.
        output_filename: String,
        // /// column name containing geometry
        // #[arg(long, default_value_t = String::from("geometry"))]
        // geometry_column: String,
        // /// column name containing activity category name
        // #[arg(long)]
        // category_column: String,
        // /// the format of the category type
        // #[arg(long, default_value_t = CategoryFormat::String)]
        // category_format: CategoryFormat,
        /// comma-delimited list of categories to keep
        #[arg(long)]
        category_filter: String,
    },
    #[command(
        name = "overlay",
        about = "aggregate a bambam output to some other geospatial dataset via some overlay operation"
    )]
    OutputOverlay {
        /// a CSV file containing a bambam output
        mep_matrix_filename: String,
        /// a file containing WKT geometries tagged with ids
        overlay_filename: String,
        /// file path to write the result dataset
        output_filename: String,
        /// overlay method to apply
        #[arg(long, default_value_t = OverlayOperation::Intersection)]
        how: OverlayOperation,
        /// name of geometry column in the overlay file
        #[arg(long, default_value_t = String::from("geometry"))]
        geometry_column: String,
        /// name of the id column in the overlay file
        #[arg(long, default_value_t = String::from("GEOID"))]
        id_column: String,
    },
}

impl App {
    pub fn run(&self) -> Result<(), String> {
        env_logger::init();
        match self {
            Self::OutputOverlay {
                mep_matrix_filename,
                overlay_filename,
                output_filename,
                how,
                geometry_column,
                id_column,
            } => overlay::run(
                mep_matrix_filename,
                overlay_filename,
                output_filename,
                how,
                geometry_column,
                id_column,
            ),
            Self::OpportunityVectorization {
                vertices_compass_filename,
                opportunities_filename,
                output_filename,
                source_format,
                category_filter,
            } => oppvec::run(
                vertices_compass_filename,
                opportunities_filename,
                output_filename,
                source_format,
                category_filter,
            ),
        }
    }
}

fn main() {
    let args = CliArgs::parse();
    args.app.run().unwrap();
}

#[cfg(test)]
mod tests {
    use bambam::app::oppvec;

    #[allow(unused)]
    #[ignore = "working test case"]
    fn test_vec() {
        let result = oppvec::run(
            &String::from("/Users/rfitzger/dev/nrel/routee/routee-compass-tomtom/data/tomtom_denver/vertices-compass.csv.gz"),
            &String::from("/Users/rfitzger/data/mep/mep3/input/opportunities/us-places.csv"),
            &String::from(""),
            &bambam::app::oppvec::SourceFormat::OvertureMaps { geometry_column: None, category_column: None },
            &String::from("eat_and_drink,retail,health_and_medical,public_service_and_government,arts_and_entertainment"),
        );
        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
