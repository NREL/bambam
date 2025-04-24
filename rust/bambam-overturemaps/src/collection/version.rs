#[allow(unused)]
pub enum ReleaseVersion{
    Jan2025,
    Feb2025,
    Mar2025,
    Latest,
    Custom{ version: String }
}


impl From<ReleaseVersion> for String{
    fn from(version: ReleaseVersion) -> Self{
        match version {
            ReleaseVersion::Jan2025 => "2025-01-22.0".into(),
            ReleaseVersion::Feb2025 => "2025-02-19.0".into(),
            ReleaseVersion::Mar2025 => "2025-03-19.0".into(),
            ReleaseVersion::Custom { version } => version,
            ReleaseVersion::Latest => "latest".into()
        }
    }
}