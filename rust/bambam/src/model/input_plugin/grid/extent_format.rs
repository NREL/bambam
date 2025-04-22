use geo::Geometry;
use routee_compass_core::config::ConfigJsonExtensions;
use serde::{Deserialize, Serialize};
use wkt;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtentFormat {
    /// user extent field to be treated as a WKT
    #[default]
    Wkt,
    // future extention points:
    // Wkb,
    // GeoJson,
}

impl ExtentFormat {
    pub fn get_extent(&self, input: &mut serde_json::Value) -> Result<Geometry, String> {
        match self {
            ExtentFormat::Wkt => input
                .get_config_serde(&super::EXTENT, &"<root>")
                .map_err(|e| {
                    format!(
                        "failure reading extent, are you sure you submitted a valid WKT?: {}",
                        e
                    )
                }), // todo:
                    //   this fails with an explicit panic: thread 'main' panicked at /Users/rfitzger/.cargo/registry/src/index.crates.io-6f17d22bba15001f/wkb-0.7.1/src/lib.rs:338:14
                    //   which is a peek method checking for big vs little endianness
                    //   but we are somehow getting a value that isn't 1 or 2
                    // ExtentFormat::Wkb => {
                    //     let extent_string = input
                    //         .get_config_string(&super::EXTENT, &"")
                    //         .map_err(|e| format!("expected extent WKB: {}", e))?;
                    //     let mut c = extent_string.as_bytes();
                    // wkb::wkb_to_geom(&mut c)
                    //         .map_err(|e| format!("failure converting wkb to geo: {:?}", e))
                    // }
        }
    }
}
