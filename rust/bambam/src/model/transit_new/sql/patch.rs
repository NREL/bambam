// #[derive(Serialize, Deserialize)]
// pub enum Patch {
//     #[serde(deserialize_with = "deserialize_num")]
//     Simple(u64),
//     #[serde(deserialize_with = "deserialize_str")]
//     WithSuffix(String),
// }

// fn deserialize_num<'a, D>(de: &mut D) -> Result<Patch, D::Error>
// where
//     D: de::Deserializer<'a>,
// {
//     let result: serde_json::Value = serde::Deserialize::deserialize(de)?;
//     match result {
//         serde_json::Value::Number(ref n) => {
//             let patch = n.as_u64().ok_or(de::Error::custom(format!(
//                 "value deserialized as number is not a nonnegative integer: {}",
//                 n
//             )))?;
//             Ok(Patch::Simple(patch))
//         }
//         _ => Err(de::Error::custom(format!("invalid numeric '{}'", result))),
//     }
// }

// fn deserialize_str<D>(de: &mut D) -> Result<Patch, D::Error>
// where
//     D: serde::Deserializer,
// {
//     let result: json::Value = serde::Deserialize::deserialize(de)?;
//     match result {
//         serde_json::Value::Number(ref n) => {
//             let patch = n.as_u64().ok_or(de::Error::custom(format!(
//                 "value deserialized as number is not a nonnegative integer: {}",
//                 n
//             )))?;
//             Ok(Patch::Simple(patch))
//         }
//         serde_json::Value::String(ref s) => Ok(Patch::WithSuffix(s.clone())),
//         _ => Err(de::Error::custom("Internal Error")),
//     }
// }

// impl Display for Patch {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Patch::Simple(patch) => write!(f, "{}", patch),
//             Patch::WithSuffix(patch) => write!(f, "{}", patch),
//         }
//     }
// }

// impl FromStr for Patch {
//     type Err = String;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         serde_json::from_str::<Patch>(s)
//             .map_err(|e| format!("semver patch deserialization error: {}", e))
//     }
// }
