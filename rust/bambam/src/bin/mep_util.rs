use bambam::app::{
    oppvec::{self, oppvec_ops},
    overlay::{self, OverlayOperation},
};
use clap::{Parser, Subcommand};
use itertools::Itertools;
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

        // /// column name containing activity category name
        #[arg(long)]
        category_column: String,

        // /// optional column name containing activity counts. if omitted, counts each row as 1 opportunity.
        #[arg(long)]
        count_column: Option<String>,
        // /// the format of the category type
        // #[arg(long, default_value_t = CategoryFormat::String)]
        // category_format: CategoryFormat,
        // / comma-delimited list of categories to keep
        #[arg(long)]
        activity_categories: String,
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
        /// comma-delimited list of categories to keep
        #[arg(long)]
        activity_categories: String,
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
            Self::OpportunitiesLongFormat {
                vertices_compass_filename,
                opportunities_filename,
                output_filename,
                geometry_column,
                x_column,
                y_column,
                category_column,
                count_column,
                activity_categories,
            } => {
                let geometry_format = oppvec::GeometryFormat::new(
                    geometry_column.as_ref(),
                    x_column.as_ref(),
                    y_column.as_ref(),
                )?;
                let source_format = oppvec::SourceFormat::LongFormat {
                    geometry_format,
                    category_column: category_column.clone(),
                    count_column: count_column.clone(),
                };
                let cats = activity_categories
                    .split(",")
                    .map(|c| c.to_owned())
                    .collect_vec();
                oppvec::run(
                    vertices_compass_filename,
                    opportunities_filename,
                    output_filename,
                    &source_format,
                    &cats,
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
                activity_categories,
            } => {
                let geometry_format = oppvec::GeometryFormat::new(
                    geometry_column.as_ref(),
                    x_column.as_ref(),
                    y_column.as_ref(),
                )?;

                let column_mapping = oppvec_ops::create_column_mapping(column_mapping)?;
                let source_format = oppvec::SourceFormat::WideFormat {
                    geometry_format,
                    column_mapping,
                };
                let cats = activity_categories
                    .split(",")
                    .map(|c| c.to_owned())
                    .collect_vec();
                oppvec::run(
                    vertices_compass_filename,
                    opportunities_filename,
                    output_filename,
                    &source_format,
                    &cats,
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
    use bambam::app::oppvec;

    #[allow(unused)]
    #[ignore = "working test case"]
    fn test_vec() {
        let result = oppvec::run(
            &String::from("/Users/rfitzger/dev/nrel/routee/routee-compass-tomtom/data/tomtom_denver/vertices-compass.csv.gz"),
            &String::from("/Users/rfitzger/data/mep/mep3/input/opportunities/us-places.csv"),
            "",
            &oppvec::SourceFormat::LongFormat { geometry_format: oppvec::GeometryFormat::XYColumns { x_column: String::from("longitude"), y_column: String::from("latitude") }, category_column: String::from("activity_type"), count_column: None},
            &[String::from("retail"),String::from("entertainment"),String::from("healthcare"),String::from("services"),String::from("food")],
        );
        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
