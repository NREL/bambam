use serde::{Deserialize, Serialize};
use crate::model::output_plugin::isochrone::{isochrone_algorithm::IsochroneAlgorithm, isochrone_output_format::IsochroneOutputFormat, time_bin_type::TimeBinType, DestinationPointGeneratorConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IsochroneOutputPluginConfig {
    pub time_bin: TimeBinType,
    pub isochrone_algorithm: IsochroneAlgorithm,
    pub isochrone_output_format: IsochroneOutputFormat,
    pub destination_point_generator: DestinationPointGeneratorConfig
}