use routee_compass_core::model::unit::{AsF64, Time, TimeUnit};

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
    pub fn departure_list_query(departure_time: (&Time, &TimeUnit)) -> Departure {
        let (time, time_unit) = departure_time;
        let t_secs = time_unit.convert(time, &TimeUnit::Seconds);
        let secs = t_secs.to_f64() as u32;
        Departure {
            time_seconds: secs,
            duration_seconds: 0,
        }
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
    let (time, time_unit) = value;
    let t_secs = time_unit.convert(time, &TimeUnit::Seconds);
    if t_secs < Time::ZERO || 86400.0 < t_secs.as_f64() {
        Err(format!(
            "invalid departure time {} must be in seconds range [0,86400)",
            t_secs
        ))
    } else {
        Ok(t_secs.to_f64() as u32)
    }
}

fn create_leg_duration_internal(value: (&Time, &TimeUnit)) -> Result<u16, String> {
    let (time, time_unit) = value;
    let t_secs = time_unit.convert(time, &TimeUnit::Seconds);
    if t_secs < Time::ZERO || 65536.0 < t_secs.as_f64() {
        Err(format!(
            "invalid trip leg duration {} must be in seconds range [0,65536)",
            t_secs
        ))
    } else {
        Ok(t_secs.to_f64() as u16)
    }
}
