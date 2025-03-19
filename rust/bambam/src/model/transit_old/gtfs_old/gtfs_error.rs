use zip::result::ZipError;

#[derive(thiserror::Error, Debug)]
pub enum GtfsError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ZipError(#[from] ZipError),
}
