use regex::RegexBuilder;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(try_from = "String")]
pub struct DotDelimitedPath(String);

impl DotDelimitedPath {
    pub const DOT_DELIMITED_REGEX: &str = "[A-Za-Z.]+";
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
    pub fn as_jsonpath(&self) -> String {
        prepend_if_missing(&self.0, "$.")
    }

    pub fn as_json_pointer(&self) -> String {
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
