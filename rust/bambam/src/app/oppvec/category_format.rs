use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Clone, Debug, ValueEnum)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CategoryFormat {
    String,
    OvertureMaps,
}

impl std::fmt::Display for CategoryFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CategoryFormat::String => write!(f, "string"),
            CategoryFormat::OvertureMaps => {
                write!(f, "overture_maps")
            }
        }
    }
}

impl CategoryFormat {
    fn description(&self) -> String {
        match self {
            CategoryFormat::String => String::from("string read directly from CSV cell"),
            CategoryFormat::OvertureMaps => String::from(
                r#"overture_maps category format is a json object with root parent at '.alternate[0]' position,
                    which is the most general category for this entry. for example, a
                    record with primary entry 'elementary_school' will have a '.alternate[0]' value of 'school'"#,
            ),
        }
    }

    pub fn read(&self, value: &str) -> Result<Option<String>, String> {
        // log::debug!("CategoryFormat::read with '{}'", value);
        match self {
            CategoryFormat::String if value.is_empty() => Ok(None),
            CategoryFormat::String => Ok(Some(String::from(value))),
            CategoryFormat::OvertureMaps => {
                log::debug!("read with value '{}'", value);
                let json: Value = serde_json::from_str(value).map_err(|e| format!("{}", e))?;
                match json {
                    Value::Null => Ok(None),
                    Value::Object(map) => {
                        log::debug!("object to pull from: {}", value);

                        pull_top_level_category(&map).map_err(|e| format!("{}: {}", e, value))
                    }
                    _ => Err(format!("value is not a JSON object or null: {}", value)),
                }
            }
        }
    }
}

fn pull_top_level_category(map: &Map<String, Value>) -> Result<Option<String>, String> {
    let primary = map
        .get("primary")
        .ok_or_else(|| String::from("row is not a JSON object with a 'primary' key"))?;
    // 'primary' may be an array or null
    match primary {
        Value::Null => Ok(None),
        Value::String(string) => Ok(Some(string.clone())),
        _ => Err(format!(
            "'primary' entry is not a string or null as expected, instead found {}",
            primary
        )),
    }

    // let alternate = map
    //     .get("alternate")
    //     .ok_or_else(|| String::from("row is not a JSON object with a 'alternate' key"))?;
    // // 'alternate' may be an array or null
    // let alternate_arr = match alternate {
    //     Value::Null => return Ok(None),
    //     Value::Array(values) => values,
    //     _ => {
    //         return Err(format!(
    //             "'alternate' entry is not an array or null as expected, instead found {}",
    //             alternate
    //         ))
    //     }
    // };
    // let category = alternate_arr
    //     .get(0)
    //     .ok_or_else(|| String::from("found array at '.alternate[0]', but it is empty"))?;
    // let cat_str = category
    //     .as_str()
    //     .ok_or_else(|| String::from("could not decode map at '.alternate[0]' as string"))?;
    // Ok(Some(String::from(cat_str)))
}
