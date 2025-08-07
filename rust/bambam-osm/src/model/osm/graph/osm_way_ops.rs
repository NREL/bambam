use std::borrow::Cow;

use crate::model::osm::graph::{OsmNodeId, OsmWayDataSerializable};
use geo::LineString;
use itertools::Itertools;
use routee_compass_core::model::unit::{Convert, Speed, SpeedUnit};
use serde::{de, Serializer};
use wkt::ToWkt;

pub const DEFAULT_WALK_SPEED_KPH: f64 = 5.0;

/// deals with the various ways that speed keys can appear. handles
/// valid cases such as:
///   - 45        (45 kph)
///   - 45 mph    (72.4203 kph)
///   - walk      (5 kph)
///
/// and invalid cases that are documented, such as:
///   - 45; 80    (takes the smaller of the two, so, 45 kph)
///
/// see https://wiki.openstreetmap.org/wiki/Key:maxspeed
pub fn deserialize_speed(
    s: &str,
    separator: Option<&str>,
    ignore_invalid_entries: bool,
) -> Result<Option<(Speed, SpeedUnit)>, String> {
    let separated_entries = match separator {
        Some(sep) => s.split(sep).collect_vec(),
        None => vec![s],
    };
    match separated_entries[..] {
        [] => Err(format!(
            "internal error: attempting to unpack empty maxspeed value '{s}'"
        )),
        [entry] => {
            match entry.split(" ").collect_vec()[..] {
                // see https://wiki.openstreetmap.org/wiki/Key:maxspeed#Possible_tagging_mistakes
                // for list of some values we should ignore that are known.
                ["unposted"] => Ok(None),
                ["unknown"] => Ok(None),
                ["default"] => Ok(None),
                ["variable"] => Ok(None),
                ["national"] => Ok(None),
                ["25mph"] => Ok(Some((Speed::from(25.0), SpeedUnit::MPH))),

                // todo! handle all default speed limits
                // see https://wiki.openstreetmap.org/wiki/Default_speed_limits
                ["walk"] => {
                    // Austria + Germany's posted "walking speed". i found a reference that
                    // suggests this is 4-7kph:
                    // https://en.wikivoyage.org/wiki/Driving_in_Germany#Speed_limits
                    Ok(Some((Speed::from(DEFAULT_WALK_SPEED_KPH), SpeedUnit::KPH)))
                }
                [speed_str] => {
                    let speed_result = speed_str
                        .parse::<i64>()
                        .map(|i| i as f64)
                        .map_err(|e| format!("speed value {speed_str} not a valid number: {e}"))
                        .or_else(|e1| {
                            speed_str.parse::<f64>().map_err(|e2| {
                                format!("speed value {speed_str} not a valid number: {e1} {e2}")
                            })
                        });

                    let speed = match speed_result {
                        Ok(speed) => speed,
                        Err(e) if !ignore_invalid_entries => {
                            return Err(e);
                        }
                        Err(_) => return Ok(None),
                    };
                    if speed == 0.0 || speed.is_nan() {
                        Ok(None)
                    } else {
                        Ok(Some((Speed::from(speed), SpeedUnit::KPH)))
                    }
                }
                [speed_str, unit_str] => {
                    let speed_result = speed_str
                        .parse::<f64>()
                        .map_err(|e| format!("speed value {speed_str} not a valid number: {e}"));

                    let speed = match speed_result {
                        Ok(speed) => speed,
                        Err(e) if !ignore_invalid_entries => {
                            return Err(e);
                        }
                        Err(_) => return Ok(None),
                    };
                    if speed == 0.0 || speed.is_nan() {
                        return Ok(None);
                    }
                    let speed_unit = match unit_str {
                        "kph" => SpeedUnit::KPH,
                        "mph" => SpeedUnit::MPH,
                        _ if !ignore_invalid_entries => {
                            return Err(format!(
                                "unknown speed unit {unit_str} with value {speed}"
                            ));
                        }
                        _ => {
                            // some garbage or uncommon unit type like feet per minute, we can skip this entry.
                            return Ok(None);
                        }
                    };
                    let result = (Speed::from(speed), speed_unit);
                    Ok(Some(result))
                }
                _ => Err(format!("unexpected maxspeed entry '{s}'")),
            }
        }
        _ => {
            let maxspeeds = separated_entries
                .to_vec()
                .iter()
                .map(|e| deserialize_speed(e, separator, ignore_invalid_entries))
                .collect::<Result<Vec<_>, _>>()?;
            let min = maxspeeds
                .into_iter()
                .min_by_key(|m| match m {
                    Some((s, su)) => {
                        let mut s_cow = Cow::Borrowed(s);
                        match su.convert(&mut s_cow, &SpeedUnit::KPH) {
                            Ok(()) => s_cow.into_owned(),
                            Err(_) => Speed::from(999999.9),
                        }
                    }
                    None => Speed::from(999999.9),
                })
                .flatten();
            Ok(min)
        }
    }
}

/// deserializes a CSV string, which should be enquoted, into a LineString<f32>.
pub fn csv_string_to_linestring(v: &str) -> Result<LineString<f32>, String> {
    // Remove surrounding double quotes if present
    let cleaned_v = if v.starts_with('"') && v.ends_with('"') && v.len() > 1 {
        &v[1..v.len() - 1]
    } else {
        v
    };

    let wkt: wkt::Wkt<f32> = cleaned_v
        .parse()
        .map_err(|e| format!("failed to parse WKT string: {e}"))?;
    let linestring: LineString<f32> = wkt
        .try_into()
        .map_err(|e| format!("failed to parse WKT string: {e}"))?;
    Ok(linestring)
}

/// uses a WKT geometry representation to serialize geo::LineString types
pub fn serialize_linestring<S>(row: &LineString<f32>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let wkt = row.to_wkt().to_string();
    s.serialize_str(&wkt)
}

/// writes geo::LineString types as a WKT
pub fn deserialize_linestring<'de, D>(d: D) -> Result<LineString<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct LineStringVisitor;

    impl<'de> de::Visitor<'de> for LineStringVisitor {
        type Value = LineString<f32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an enquoted WKT LineString")
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            csv_string_to_linestring(&v).map_err(serde::de::Error::custom)
        }
    }

    d.deserialize_string(LineStringVisitor {})
}

/// takes all node ids found between src an dst in a list of nodes.
/// node list is not required to start with src, end with dst.
pub fn extract_between_nodes<'a>(
    src: &'a OsmNodeId,
    dst: &'a OsmNodeId,
    nodes: &'a [OsmNodeId],
) -> Option<&'a [OsmNodeId]> {
    let start = nodes.iter().position(|x| x == src)?; // Using ? for early return
    let end = nodes[start..].iter().position(|x| x == dst)?; // Search after 'a'

    if start <= start + end {
        Some(&nodes[start..=start + end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::model::osm::graph::{osm_way_ops, OsmNodeId};
    use routee_compass_core::model::unit::{AsF64, SpeedUnit};

    #[test]
    fn test_extract() {
        let nodes = vec![
            OsmNodeId(1),
            OsmNodeId(2),
            OsmNodeId(3),
            OsmNodeId(4),
            OsmNodeId(5),
            OsmNodeId(6),
        ];
        let result = osm_way_ops::extract_between_nodes(&OsmNodeId(2), &OsmNodeId(4), &nodes);
        println!("{result:?}");
        let expected = [&OsmNodeId(2), &OsmNodeId(3), &OsmNodeId(4)];
        match result {
            Some([a, b, c]) => {
                assert_eq!(a, &nodes[1]);
                assert_eq!(b, &nodes[2]);
                assert_eq!(c, &nodes[3]);
            }
            _ => panic!("not as expected"),
        }
    }

    #[test]
    fn deserialize_speed_1() {
        //   - 45        (45 kph)
        match osm_way_ops::deserialize_speed("45", None, false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 45.0);
                assert_eq!(speed_unit, SpeedUnit::KPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }
    #[test]
    fn deserialize_speed_2() {
        //   - 45 mph    (72.4203 kph)
        match osm_way_ops::deserialize_speed("45 mph", None, false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 45.0);
                assert_eq!(speed_unit, SpeedUnit::MPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }
    #[test]
    fn deserialize_speed_3() {
        //   - walk      (5 kph)
        match osm_way_ops::deserialize_speed("5 kph", None, false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 5.0);
                assert_eq!(speed_unit, SpeedUnit::KPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn deserialize_speed_sep_1() {
        //   - a few speed values, where 3 kph is the minimum
        match super::deserialize_speed("3.1415 kph;3;2 mph", Some(";"), false) {
            Ok(Some((speed, speed_unit))) => {
                assert_eq!(speed.as_f64(), 3.0); // using a pessimistic approach, picks the min speed in the group
                assert_eq!(speed_unit, SpeedUnit::KPH);
            }
            Ok(None) => panic!("should parse valid speed"),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn deserialize_csv_linestring_01() {
        let wkt = "\"LINESTRING (0 0, 1 1)\"";
        let expected = geo::line_string![
            geo::coord! { x: 0.0f32, y: 0.0f32},
            geo::coord! { x: 1.0f32, y: 1.0f32},
        ];
        match super::csv_string_to_linestring(wkt) {
            Ok(result) => assert_eq!(result, expected),
            Err(e) => panic!("{e}"),
        }
    }

     #[test]
    fn deserialize_csv_linestring_no_quotes() {
        let wkt = "LINESTRING (0 0, 1 1)";
        let expected = geo::line_string![
            geo::coord! { x: 0.0f32, y: 0.0f32},
            geo::coord! { x: 1.0f32, y: 1.0f32},
        ];
        match super::csv_string_to_linestring(wkt) {
            Ok(result) => {},
            Err(e) => panic!("{e}"),
        }
    }
}
