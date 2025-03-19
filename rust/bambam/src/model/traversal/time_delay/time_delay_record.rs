use geo::{ConvexHull, Geometry, MultiPolygon};
use geo::{Point, Polygon};
use routee_compass_core::model::unit::Time;
use rstar::{RTreeObject, AABB};
use serde::de;
use serde::Serialize;
use wkt::TryFromWkt;

#[derive(Serialize, Clone, Debug)]
pub struct TimeDelayRecord {
    pub geometry: Geometry<f32>,
    pub time: Time,
}

impl RTreeObject for TimeDelayRecord {
    type Envelope = AABB<Point<f32>>;
    fn envelope(&self) -> Self::Envelope {
        match &self.geometry {
            Geometry::Polygon(p) => p.envelope(),
            Geometry::MultiPolygon(mp) => mp.convex_hull().envelope(),
            Geometry::GeometryCollection(gc) => gc.convex_hull().envelope(),
            _ => panic!("only polygon, multipolygon, and geometry collection are supported"),
        }
    }
}

/// custom deserializer for access records which expects a
/// geometry and time field. the geometry should be a WKT POLYGON
/// or MULTIPOLYGON, and the time value should be a real number.
impl<'de> de::Deserialize<'de> for TimeDelayRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RecordVisitor;

        impl<'de> de::Visitor<'de> for RecordVisitor {
            type Value = TimeDelayRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an enquoted WKT string, a comma, and a number")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut geometry_option: Option<Geometry<f32>> = None;
                let mut time_option: Option<Time> = None;
                let mut next: Option<(&str, &str)> = map.next_entry()?;
                while next.is_some() {
                    if let Some((key, value)) = next {
                        match key {
                            "geometry" => {
                                // should be one of POLYGON | MULTIPOLYGON
                                let value_trim = value.replace('\"', "");
                                let polygon_result = Polygon::try_from_wkt_str(value_trim.as_str())
                                    .map(|p: Polygon<f32>| Geometry::Polygon(p));
                                let row_geometry = polygon_result
                                    .or_else(|_| {
                                        MultiPolygon::try_from_wkt_str(value)
                                            .map(Geometry::MultiPolygon)
                                    })
                                    .map_err(|e| {
                                        de::Error::custom(format!(
                                            "unable to parse WKT geometry '{}': {}",
                                            &value, e
                                        ))
                                    })?;
                                geometry_option = Some(row_geometry);
                            }
                            "time" => {
                                let row_time =
                                    serde_json::from_str::<Time>(value).map_err(|e| {
                                        de::Error::custom(format!(
                                            "unable to parse time value '{}': {}",
                                            &value, e
                                        ))
                                    })?;
                                time_option = Some(row_time);
                            }
                            &_ => {}
                        }
                    } else {
                        return Err(de::Error::custom("internal error"));
                    }
                    next = map.next_entry()?;
                }

                match (geometry_option, time_option) {
                    (None, None) => Err(de::Error::missing_field("geometry,time")),
                    (None, Some(_)) => Err(de::Error::missing_field("geometry")),
                    (Some(_), None) => Err(de::Error::missing_field("time")),
                    (Some(geometry), Some(time)) => Ok(TimeDelayRecord { geometry, time }),
                }
            }
        }

        deserializer.deserialize_map(RecordVisitor {})
    }
}
