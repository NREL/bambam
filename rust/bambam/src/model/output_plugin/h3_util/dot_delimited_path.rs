use regex::RegexBuilder;
use serde::Serialize;

/// a dot-delimited path is a JSON path delimited by '.' characters, such as
/// `parent.child.grandchild`. this format is used over JSONPath and JSON Pointer
/// to describe a path to a location but can be converted to JSONPath or JSON Pointer
/// as needed for compatibility.
#[derive(Clone, Debug, Serialize)]
#[serde(try_from = "String")]
pub struct DotDelimitedPath(String);

impl DotDelimitedPath {
    /// a pattern to match a dot-delimited string
    pub const DOT_DELIMITED_REGEX: &str = r"^[a-zA-Z_]+(\.[a-zA-Z_]+)*$";
}

impl TryFrom<String> for DotDelimitedPath {
    type Error = String;

    /// maps from strings but only for dot-delimited names that are non-numeric alpha strings
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let regex = RegexBuilder::new(Self::DOT_DELIMITED_REGEX)
            .build()
            .map_err(|e| e.to_string())?;
        if regex.is_match(&value) {
            Ok(Self(value))
        } else {
            Err(format!(
                "String '{}' does not match pattern '{}'",
                value,
                Self::DOT_DELIMITED_REGEX
            ))
        }
    }
}

impl DotDelimitedPath {
    /// converts this dot-delimited path to a JSONPath by simply adding the root prefix $.
    pub fn to_jsonpath(&self) -> String {
        prepend_if_missing(&self.0, "$.")
    }

    /// converts this dot-delimited path to a JSON Pointer by simply adding the root
    /// prefix '/' and replacing all dots '.' with '/'.
    pub fn to_jsonpointer(&self) -> String {
        prepend_if_missing(&self.0.replace(".", "/"), "/")
    }
}

fn prepend_if_missing(path: &str, prefix: &str) -> String {
    if !path.starts_with(prefix) {
        format!("{prefix}{path}")
    } else {
        path.to_string()
    }
}
