use std::fs::File;

use zip::ZipArchive;

use super::gtfs_error::GtfsError;

pub struct Archive {}

impl Archive {
    pub fn new(_zip_archive: &ZipArchive<&File>) -> Result<Archive, GtfsError> {
        todo!()
    }
}
