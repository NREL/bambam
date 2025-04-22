use itertools::Itertools;
use osmpbf::Element;
use regex::Regex;
use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{
    collections::HashSet,
    fmt::{self, Display},
    str::FromStr,
};

use super::FilterOp;

#[derive(Debug, Clone)]
/// represents a single fragment of an overpass API filter query
/// see <https://wiki.openstreetmap.org/wiki/Overpass_API/Language_Guide#Tag_request_clauses_(or_%22tag_filters%22)>
pub struct FilterQuery {
    /// the key in the tag's key/value pair to match against
    tag: String,
    /// operation/predicate used on this query
    op: FilterOp,
    /// the values we are expecting. if empty, then simply
    /// any value set at this tag returns true (used to represent
    /// existential queries such as "highway:*").
    values: HashSet<String>,
}

impl FilterQuery {
    pub fn filter(&self, element: &Element<'_>) -> bool {
        let tag = match element {
            Element::Node(node) => node.tags().find(|(k, v)| *k == self.tag),
            Element::DenseNode(dense_node) => dense_node.tags().find(|(k, v)| *k == self.tag),
            Element::Way(way) => way.tags().find(|(k, v)| *k == self.tag),
            Element::Relation(relation) => relation.tags().find(|(k, v)| *k == self.tag),
        };
        match tag {
            None => false,
            Some((_, value)) => match self.op {
                FilterOp::Equals => self.values.contains(value),
                FilterOp::NotEquals => !self.values.contains(value),
            },
        }
    }
}

impl Display for FilterQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let values = self.values.iter().join("|");
        write!(f, "['{}']{}['{}']", self.tag, self.op, values)
    }
}

impl FilterQuery {
    const QUERY_REGEX: &str = "\\[\"([\\w:*]+)\"(=|~)\"([\\w|]+)\"\\]";
}

impl FromStr for FilterQuery {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // regex here should be built at compile time
        let re = Regex::new(Self::QUERY_REGEX)
            .map_err(|e| format!("internal error building overpass query regex: {}", e))?;
        match re.captures(s) {
            None => Err(format!("unable to parse overpass query: '{}'", s)),
            Some(groups) => {
                let tag = String::from(&groups[0]);
                let op = FilterOp::from_str(&groups[1])?;
                let values = groups[2]
                    .split("|")
                    .map(String::from)
                    .collect::<HashSet<_>>();
                let result = FilterQuery { tag, op, values };
                Ok(result)
            }
        }
    }
}

struct OverpassFilterQueryVisitor;

impl Visitor<'_> for OverpassFilterQueryVisitor {
    type Value = FilterQuery;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid overpass filter query. see https://wiki.openstreetmap.org/wiki/Overpass_API/Language_Guide#Tag_request_clauses_(or_%22tag_filters%22).")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        FilterQuery::from_str(v).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for FilterQuery {
    fn deserialize<D>(deserializer: D) -> Result<FilterQuery, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i32(OverpassFilterQueryVisitor)
    }
}

impl Serialize for FilterQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
