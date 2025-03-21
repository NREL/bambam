use std::borrow::Cow;

use routee_compass_core::model::unit::{AsF64, Convert, Time, TimeUnit, UnitError};

/// represents a single departure time for a static scheduled route.
#[derive(Clone, Debug)]
pub struct Departure {
    time_seconds: u32,     // >> 24 hours
    duration_seconds: u16, // ~ 18 hours
}

impl Departure {
    pub fn new(
        departure_time: (&Time, &TimeUnit),
        leg_duration: (&Time, &TimeUnit),
    ) -> Result<Departure, String> {
        let time_seconds = create_departure_time_internal(departure_time)?;
        let duration_seconds = create_leg_duration_internal(leg_duration)?;
        Ok(Departure {
            time_seconds,
            duration_seconds,
        })
    }
    /// OrderedSkipList.upper_bound() query value must be of type `Departure`.
    /// this creates a 'dummy' value with the matching departure time.
    pub fn departure_list_query(departure_time: (&Time, &TimeUnit)) -> Result<Departure, String> {
        let (time, time_unit) = departure_time;
        let mut t_secs = Cow::Borrowed(time);
        time_unit
            .convert(&mut t_secs, &TimeUnit::Seconds)
            .map_err(|e| e.to_string())?;
        let secs = t_secs.as_f64() as u32;
        let result = Departure {
            time_seconds: secs,
            duration_seconds: 0,
        };
        Ok(result)
    }
}

impl PartialEq for Departure {
    fn eq(&self, other: &Self) -> bool {
        self.time_seconds == other.time_seconds
    }
}

impl PartialOrd for Departure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time_seconds.partial_cmp(&other.time_seconds)
    }
}

fn create_departure_time_internal(value: (&Time, &TimeUnit)) -> Result<u32, String> {
    let secs = to_seconds_bounded(value, (None, &Time::from(86400.0)))?;
    Ok(secs as u32)
}

fn create_leg_duration_internal(value: (&Time, &TimeUnit)) -> Result<u16, String> {
    let secs = to_seconds_bounded(value, (None, &Time::from(65536.0)))?;
    Ok(secs as u16)
}

fn to_seconds_bounded(
    value: (&Time, &TimeUnit),
    bounds: (Option<&Time>, &Time),
) -> Result<f64, String> {
    let (min_value, max) = bounds;
    let min = min_value.unwrap_or(&Time::ZERO);
    let (time, time_unit) = value;
    let mut t_convert = Cow::Borrowed(time);
    time_unit
        .convert(&mut t_convert, &TimeUnit::Seconds)
        .map_err(|e| e.to_string())?;
    let t_secs = t_convert.as_ref();
    if t_secs < min || max < t_secs {
        Err(format!(
            "invalid number of seconds {} is outside of range [{},{})",
            t_secs, min, max
        ))
    } else {
        Ok(t_secs.as_f64())
    }
}
