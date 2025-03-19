use bambam::app::{
    matoverlay::{self, Overlay},
    oppvec::{self, CategoryFormat},
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
    OpportunityVectorization {
        #[arg(long)]
        vertices_compass_filename: String,
        #[arg(long)]
        opportunities_filename: String,
        #[arg(long)]
        output_directory: String,
        #[arg(long, default_value_t = String::from("geometry"))]
        geometry_column: String,
        #[arg(long)]
        category_column: String,
        #[arg(long, default_value_t = CategoryFormat::String)]
        category_format: CategoryFormat,
        #[arg(long, help = "comma-delimited list of categories to keep")]
        category_filter: String,
    },
    MepMatrixOverlay {
        #[arg(long)]
        mep_matrix_filename: String,
        #[arg(long)]
        overlay_filename: String,
        #[arg(long, default_value_t = Overlay::Intersection)]
        how: Overlay,
        #[arg(long, default_value_t = String::from("geometry"))]
        geometry_column: String,
        #[arg(long, default_value_t = String::from("GEOID"))]
        id_column: String,
    },
}

impl App {
    pub fn run(&self) -> Result<(), String> {
        env_logger::init();
        match self {
            Self::MepMatrixOverlay {
                mep_matrix_filename,
                overlay_filename,
                how,
                geometry_column,
                id_column,
            } => matoverlay::run(
                mep_matrix_filename,
                overlay_filename,
                how,
                geometry_column,
                id_column,
            ),
            Self::OpportunityVectorization {
                vertices_compass_filename,
                opportunities_filename,
                output_directory,
                geometry_column,
                category_column,
                category_format,
                category_filter,
            } => oppvec::run(
                vertices_compass_filename,
                opportunities_filename,
                output_directory,
                geometry_column,
                category_column,
                category_format,
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
            &String::from("geometry"),
            &String::from("categories"),
            &bambam::app::oppvec::CategoryFormat::OvertureMaps,
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
