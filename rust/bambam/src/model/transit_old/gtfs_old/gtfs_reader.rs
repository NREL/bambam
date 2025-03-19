use super::{archive::Archive, gtfs_error::GtfsError};
use std::fs::{self, File};

/// deprecated!
pub struct GtfsReader {}

impl GtfsReader {
    /// reads any number of GTFS archives from a directory. each is expected
    /// to be stored in a .zip compressed format. the directory must not contain
    /// any additional .zip archives other than GTFS-encoded archives.
    pub fn read(&self, directory: &String) -> Result<Vec<Archive>, GtfsError> {
        let paths = fs::read_dir(directory).map_err(GtfsError::IoError)?;
        let zip_archives = paths
            .flat_map(|p| {
                if let Ok(dir_entry) = p {
                    let path = dir_entry.path();
                    if let Some(ext) = dir_entry.path().extension() {
                        if ext == "zip" {
                            let file = fs::File::open(path)?;
                            let result: Result<Option<File>, std::io::Error> = Ok(Some(file));
                            result
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        let gtfs_archives: Vec<Archive> = zip_archives
            .iter()
            .map(|file| {
                let zip_archive = zip::ZipArchive::new(file).map_err(GtfsError::ZipError)?;
                let gtfs_archive = Archive::new(&zip_archive)?;
                Ok(gtfs_archive)
            })
            .collect::<Result<Vec<_>, GtfsError>>()?;

        Ok(gtfs_archives)
    }
}
