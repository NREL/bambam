use std::borrow::Cow;

use chrono::{DateTime, Duration, Utc};
use routee_compass_core::model::unit::{AsF64, TimeUnit, UnitError};
use uom::si::f64::Time;

use super::schedule_error::ScheduleError;

/// adds time to a datetime. performs operation at the seconds resolution, so any sub-second
/// increments will be rounded off.
pub fn add_delta(
    datetime: DateTime<Utc>,
    time: Time,
) -> Result<DateTime<Utc>, ScheduleError> {
    let time_sec = time.get::<uom::si::time::second>() as i64;
    let duration = Duration::seconds(time_sec);

    // .ok_or_else(|| ScheduleError::AddTimeToDateTimeError(time, time_unit, datetime));
    let with_delta = datetime + duration;
    Ok(with_delta)
}

#[cfg(test)]
mod test {
    use chrono::{TimeZone, Timelike, Utc};
    use routee_compass_core::model::unit::{TimeUnit};
    use uom::si::f64::Time;

    use super::add_delta;

    #[test]
    fn test_add_delta() {
        let datetime = Utc.with_ymd_and_hms(2024, 1, 22, 12, 0, 0).unwrap();
        let time = Time::new::<uom::si::time::second>(3600.0);
        let result = add_delta(datetime, time).expect("should not fail");
        assert_eq!(result.hour(), 13);
    }
}
