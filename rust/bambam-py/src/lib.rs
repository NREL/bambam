use bambam::model::builders;
use pyo3::prelude::*;
use routee_compass::app::{
    bindings::CompassAppBindings,
    compass::{CompassApp, CompassAppConfig, CompassAppError, CompassBuilderInventory},
};
use routee_compass_macros::pybindings;

// add BAMBAM extensions to RouteE Compass
inventory::submit! { builders::BUILDER_REGISTRATION }

/// a wrapper around CompassApp for running BAMBAM
#[pybindings]
pub struct BambamAppWrapper {
    pub app: CompassApp,
}

impl CompassAppBindings for BambamAppWrapper {
    /// creates a BAMBAM app instance from a configuration file.
    fn from_config_toml_string(
        config_string: String,
        original_file_path: String,
    ) -> Result<Self, CompassAppError>
    where
        Self: Sized,
    {
        // while effectively the same as routee-compass-py's lib.rs,
        // this will automatically pick up both routee-compass AND bambam builders
        // during compilation, adding the access modeling extensions.
        let builder = CompassBuilderInventory::new()?;
        let config = CompassAppConfig::from_str(
            &config_string,
            &original_file_path,
            config::FileFormat::Toml,
        )?;
        let app = CompassApp::new(&config, &builder)?;
        Ok(BambamAppWrapper { app })
    }
    fn app(&self) -> &CompassApp {
        &self.app
    }
}

/// register bambam_py_api as a package in Python
#[pymodule]
fn bambam_py_api(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BambamAppWrapper>()?;
    Ok(())
}
