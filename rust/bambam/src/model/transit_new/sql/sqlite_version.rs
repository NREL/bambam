// use rusqlite::Connection;
use serde::{de, Deserialize, Serialize};
use serde_json;
use std::{fmt::Display, str::FromStr};

/// simple semver tuple. cannot "handle" prefix/suffix formats :-| wanted to enable that via
/// the "Patch" type but deserializing that got screwy. -rjf 2024-08-13
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SQLiteVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl PartialOrd for SQLiteVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.minor.partial_cmp(&other.minor) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.patch.partial_cmp(&other.patch)
    }
}

impl Display for SQLiteVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl SQLiteVersion {
    // pub fn new(conn: &Connection) -> Result<SQLiteVersion, String> {
    //     let mut version_statement = conn
    //         .prepare("select sqlite_version();")
    //         .map_err(|e| format!("failed preparing SQLite version query: {}", e))?;
    //     let mut version_response = version_statement
    //         .query([])
    //         .map_err(|e| format!("failed checking SQLite version: {}", e))?;
    //     let version: String = match version_response.next() {
    //         Ok(Some(v_res)) => v_res
    //             .get(0)
    //             .map_err(|e| format!("failure deserializing SQLite version query: {}", e)),
    //         Ok(None) => Err(String::from("failure querying SQLite version")),
    //         Err(e) => Err(format!("failure querying SQLite version: {}", e)),
    //     }?;

    //     let v_split: Vec<&str> = version.split(".").collect();
    //     let v_tuple = (v_split.get(0), v_split.get(1), v_split.get(2));
    //     match v_tuple {
    //         (Some(ma_str), Some(mi_str), Some(pa_str)) => {
    //             let major: u64 = ma_str.parse().map_err(|e| {
    //                 format!("SQLite version has malformed semver 'major' format: {}", e)
    //             })?;
    //             let minor: u64 = mi_str.parse().map_err(|e| {
    //                 format!("SQLite version has malformed semver 'minor' format: {}", e)
    //             })?;
    //             let patch: u64 = pa_str.parse().map_err(|e| {
    //                 format!("SQLite version has malformed semver 'patch' format: {}", e)
    //             })?;
    //             Ok(SQLiteVersion {
    //                 major,
    //                 minor,
    //                 patch,
    //             })
    //         }
    //         _ => Err(format!(
    //             "SQLite version has malformed semver format, found {}",
    //             version
    //         )),
    //     }
    // }
}
