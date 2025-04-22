// use rusqlite::Connection;

use crate::model::transit_new::sql::sqlite_version::SQLiteVersion;

use super::compile_option::CompileOption;

pub struct DatabaseBuilder {
    pub name: String,
    pub table: String,
}

impl Default for DatabaseBuilder {
    fn default() -> Self {
        DatabaseBuilder {
            name: String::from(DatabaseBuilder::DEFAULT_DB_NAME),
            table: String::from(DatabaseBuilder::DEFAULT_TABLE_NAME),
        }
    }
}

impl DatabaseBuilder {
    pub const DEFAULT_DB_NAME: &'static str = "mep";
    pub const DEFAULT_TABLE_NAME: &'static str = "transit";

    /// minimum supported version of SQLite is 3.24.0, which is where
    /// Rtree tables with auxiliary columns was introduced.
    /// see https://www.sqlite.org/rtree.html, section 4.1.
    pub fn min_sqlite_version() -> SQLiteVersion {
        SQLiteVersion {
            major: 3,
            minor: 24,
            patch: 0,
        }
    }

    // /// checks the version and extensions of the specified sqlite database
    // fn validate_database(&self, conn: &Connection) -> Result<(), String> {
    //     // validate SQLite version
    //     let version = SQLiteVersion::new(conn)?;
    //     let min_version = DatabaseBuilder::min_sqlite_version();
    //     if version < min_version {
    //         return Err(format!(
    //             "found sqlite version {}, does not meet minimum version
    //         requirement of {}. this is set due to the inclusion of auxiliary fields
    //         in rtrees. please install a sqlite database with at least version {} before
    //         using this library.
    //         ",
    //             version, min_version, version
    //         ));
    //     }

    //     // validate that rtree was installed
    //     let supports_rtree = CompileOption::exists(conn, "ENABLE_RTREE")?;
    //     if !supports_rtree {
    //         return Err(format!(
    //             "found sqlite database without rtree enabled. if you
    //         are installing sqlite from source, be sure to activate the compile flag
    //         for rtrees. see https://www.sqlite.org/rtree.html for more information."
    //         ));
    //     }
    //     Ok(())
    // }

    // fn create_rtree_table(&self, conn: &Connection) -> Result<(), String> {
    //     let query: String = format!(
    //         "CREATE VIRTUAL TABLE transit USING rtree(
    //     id,              -- Integer primary key
    //     minX, maxX,      -- Minimum and maximum X coordinate
    //     minY, maxY,      -- Minimum and maximum Y coordinate
    // );"
    //     );
    //     conn.execute(&query, ())
    //         .map(|_| ())
    //         .map_err(|e| format!("failed creating table: {}", e))
    // }
}

#[cfg(test)]
mod tests {
    // use super::DatabaseBuilder;
    // use rusqlite::Connection;

    // #[test]
    // fn test_db_validation() {
    //     let conn = Connection::open_in_memory().unwrap();
    //     let _ = DatabaseBuilder::default().validate_database(&conn).unwrap();
    //     ()
    // }
}
