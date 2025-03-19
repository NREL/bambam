use super::time_bin_type::TimeBinType;
use super::{
    destination_point_generator::DestinationPointGenerator,
    isochrone_algorithm::IsochroneAlgorithm, isochrone_output_format::IsochroneOutputFormat,
    isochrone_output_plugin::IsochroneOutputPlugin,
};
use routee_compass::app::compass::CompassComponentError;
use routee_compass::plugin::output::{OutputPlugin, OutputPluginBuilder};
use routee_compass::plugin::PluginError;
use routee_compass_core::config::{CompassConfigurationField, ConfigJsonExtensions};
use std::sync::Arc;

pub struct IsochroneOutputPluginBuilder {}

impl OutputPluginBuilder for IsochroneOutputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn OutputPlugin>, CompassComponentError> {
        let parent_key = CompassConfigurationField::OutputPlugins.to_string();
        let isochrone_algorithm_key = String::from("isochrone_algorithm");
        let isochrone_format_key = String::from("isochrone_output_format");
        let time_bin_type_key = String::from("time_bin");

        let time_bin_type =
            parameters.get_config_serde::<TimeBinType>(&time_bin_type_key, &parent_key)?;
        let bins = time_bin_type
            .create_bins()
            .map_err(|e| CompassComponentError::PluginError(PluginError::BuildFailed(e)))?;

        let isochrone_algorithm = parameters
            .get_config_serde::<IsochroneAlgorithm>(&isochrone_algorithm_key, &parent_key)?;
        let isochrone_output_format = parameters
            .get_config_serde::<IsochroneOutputFormat>(&isochrone_format_key, &parent_key)?;
        let destination_point_generator = parameters
            .get_config_serde::<DestinationPointGenerator>(
                &"destination_point_generator",
                &parent_key,
            )?;

        let plugin = IsochroneOutputPlugin::new(
            bins,
            isochrone_algorithm,
            isochrone_output_format,
            destination_point_generator,
        )
        .map_err(PluginError::from)?;
        Ok(Arc::new(plugin))
    }
}
