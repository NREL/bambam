use super::{extent_format::ExtentFormat, grid_input_plugin::GridInputPlugin, grid_type::GridType};
use crate::model::input_plugin::population::population_source_config::PopulationSourceConfig;
use routee_compass::plugin::input::{InputPlugin, InputPluginBuilder};
use routee_compass_core::config::{CompassConfigurationError, ConfigJsonExtensions};
use std::sync::Arc;

pub struct GridInputPluginBuilder {}

// for GridInputPlugin return 
pub fn plugin_builder(
    data: &serde_json::Value,
) -> Result<GridInputPlugin, CompassConfigurationError>{
    let pop_config: Option<PopulationSourceConfig> =
            data.get_config_serde_optional(&super::POPULATION_SOURCE, &"")?;
    let extent_format: ExtentFormat = data
        .get_config_serde(&super::EXTENT_FORMAT, &"")
        .map_err(|e| {
        CompassConfigurationError::UserConfigurationError(format!(
            "failure reading extent: {}",
            e
        ))
    })?;
    let grid_type: GridType = data
        .get_config_serde(&super::GRID_TYPE, &"")
        .map_err(|e| {
            CompassConfigurationError::UserConfigurationError(format!(
                "failure reading grid type: {}",
                e
            ))
        })?;
    let population_source = match pop_config {
        Some(conf) => conf
            .build()
            .map_err(|s| {
                let msg = format!(
                    "failure building {} configuration for grid input plugin: {}",
                    super::POPULATION_SOURCE,
                    s
                );
                CompassConfigurationError::UserConfigurationError(msg)
            })
            .map(Some),
        None => Ok(None),
    }?;

    Ok(GridInputPlugin::new(
        population_source,
        extent_format,
        grid_type,
    ))
}

// for Arc<dyn InputPlugin>
impl InputPluginBuilder for GridInputPluginBuilder {
    fn build(
        &self,
        parameters: &serde_json::Value,
    ) -> Result<Arc<dyn InputPlugin>, CompassConfigurationError> {
        let pop_config: Option<PopulationSourceConfig> =
            parameters.get_config_serde_optional(&super::POPULATION_SOURCE, &"")?;
        let extent_format: ExtentFormat = parameters
            .get_config_serde(&super::EXTENT_FORMAT, &"")
            .map_err(|e| {
            CompassConfigurationError::UserConfigurationError(format!(
                "failure reading extent: {}",
                e
            ))
        })?;
        let grid_type: GridType = parameters
            .get_config_serde(&super::GRID_TYPE, &"")
            .map_err(|e| {
                CompassConfigurationError::UserConfigurationError(format!(
                    "failure reading grid type: {}",
                    e
                ))
            })?;
        let population_source = match pop_config {
            Some(conf) => conf
                .build()
                .map_err(|s| {
                    let msg = format!(
                        "failure building {} configuration for grid input plugin: {}",
                        super::POPULATION_SOURCE,
                        s
                    );
                    CompassConfigurationError::UserConfigurationError(msg)
                })
                .map(Some),
            None => Ok(None),
        }?;

        Ok(Arc::new(GridInputPlugin::new(
            population_source,
            extent_format,
            grid_type,
        )))
    }
}
