use std::{fmt::Display, str::FromStr};

// use rusqlite::Connection;

#[derive(Debug)]
pub enum CompileOption {
    Flag(String),
    KeyValue(String, String),
}

impl FromStr for CompileOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split("=").collect::<Vec<&str>>();
        match split.as_slice() {
            [flag] => Ok(CompileOption::Flag(String::from(*flag))),
            [key, value] => Ok(CompileOption::KeyValue(
                String::from(*key),
                String::from(*value),
            )),
            _ => Err(format!("compile option not a flag or key=value: {}", s)),
        }
    }
}

impl Display for CompileOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileOption::Flag(flag) => write!(f, "{}", flag),
            CompileOption::KeyValue(k, v) => write!(f, "{}={}", k, v),
        }
    }
}

impl CompileOption {
    // /// check a compile option by name to confirm it was set (flag) or
    // /// to review the value set in a key=value pair.
    // pub fn get(conn: &Connection, key: &str) -> Result<Option<CompileOption>, String> {
    //     let mut compile_stmt = conn
    //         .prepare("SELECT * FROM pragma_compile_options WHERE compile_options = ?1;")
    //         .unwrap();
    //     let compile_opts = compile_stmt
    //         .query(rusqlite::params![key])
    //         .unwrap()
    //         .mapped(|r| {
    //             // Ok(format!("{:?}", r))
    //             let s: String = r.get(0)?;
    //             Ok(s)
    //         })
    //         .collect::<Vec<_>>();
    //     match compile_opts.as_slice() {
    //         [row] => {
    //             let row_str = row
    //                 .as_ref()
    //                 .map_err(|e| format!("failure querying compile option {}: {}", key, e))?;
    //             let option = CompileOption::from_str(&row_str)
    //                 .map_err(|e| format!("failure querying compile option {}: {}", key, e))?;
    //             Ok(Some(option))
    //         }
    //         [] => Ok(None),
    //         too_big => Err(format!(
    //             "query key '{}' returned multiple compile options: {:?}",
    //             key, too_big
    //         )),
    //     }
    // }

    // pub fn exists(conn: &Connection, key: &str) -> Result<bool, String> {
    //     CompileOption::get(conn, key).map(|result| result.is_some())
    // }

    // /// attempt to grab the value found for the given key
    // pub fn get_value(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    //     match CompileOption::get(conn, key) {
    //         Ok(Some(CompileOption::Flag(_))) => Ok(None),
    //         Ok(Some(CompileOption::KeyValue(_, v))) => Ok(Some(v.clone())),
    //         Ok(None) => Ok(None),
    //         Err(e) => Err(e),
    //     }
    // }
}
