use chrono::{DateTime, Duration, Utc};
use routee_compass_core::model::unit::{Time, TimeUnit};

/// adds time to a datetime. performs operation at the seconds resolution, so any sub-second
/// increments will be rounded off.
pub fn add_delta(datetime: DateTime<Utc>, time: Time, time_unit: TimeUnit) -> DateTime<Utc> {
    let time_sec = time_unit.convert(&time, &TimeUnit::Seconds).to_f64() as i64;
    let duration = Duration::seconds(time_sec);

    // .ok_or_else(|| ScheduleError::AddTimeToDateTimeError(time, time_unit, datetime));
    datetime + duration
}

#[cfg(test)]
mod test {
    use chrono::{TimeZone, Timelike, Utc};
    use routee_compass_core::model::unit::{Time, TimeUnit};

    use super::add_delta;

    #[test]
    fn test_add_delta() {
        let datetime = Utc.with_ymd_and_hms(2024, 1, 22, 12, 0, 0).unwrap();
        let time = Time::new(3600.0);
        let time_unit = TimeUnit::Seconds;
        let result = add_delta(datetime, time, time_unit);
        assert_eq!(result.hour(), 13);
    }
}
