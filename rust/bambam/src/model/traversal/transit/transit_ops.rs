use chrono::{Duration, NaiveDateTime};
use routee_compass_core::model::{
    state::{StateModel, StateVariable},
    traversal::TraversalModelError,
};

use crate::model::state::fieldname;

/// composes the start time and the current trip_time into a new datetime value.
pub fn get_current_time(
    start_datetime: &NaiveDateTime,
    state: &[StateVariable],
    state_model: &StateModel,
) -> Result<NaiveDateTime, TraversalModelError> {
    let trip_time = state_model
        .get_time(state, fieldname::TRIP_TIME)?
        .get::<uom::si::time::second>();
    let seconds = trip_time as i64;
    let remainder = (trip_time - seconds as f64);
    let nanos = (remainder * 1_000_000_000.0) as u32;
    let trip_duration = Duration::new(seconds, nanos).ok_or_else(|| {
        TraversalModelError::TraversalModelFailure(format!(
            "unable to build Duration from seconds, nanos: {seconds}, {nanos}"
        ))
    })?;

    let current_datetime = start_datetime.checked_add_signed(trip_duration).ok_or(
        TraversalModelError::InternalError(format!(
            "Invalid Datetime from Date {} + {} seconds",
            start_datetime, trip_time
        )),
    )?;
    Ok(current_datetime)
}

#[cfg(test)]
mod tests {
    use routee_compass_core::model::{
        state::{StateModel, StateVariable, StateVariableConfig},
        unit::TimeUnit,
    };
    use uom::si::f64::Time;

    use crate::model::state::fieldname;

    fn mock_state(time: Time, state_model: &StateModel) -> Vec<StateVariable> {
        let mut state = state_model
            .initial_state(None)
            .expect("test invariant failed: could not create initial state");
        state_model
            .set_time(&mut state, fieldname::TRIP_TIME, &time)
            .expect(&format!(
                "test invariant failed: could not set time value of {} for state",
                time.value
            ));
        state
    }

    fn mock_state_model(time_unit: Option<TimeUnit>) -> StateModel {
        let trip_time_config = StateVariableConfig::Time {
            initial: Time::new::<uom::si::time::second>(0.0),
            accumulator: true,
            output_unit: time_unit,
        };
        StateModel::new(vec![(fieldname::TRIP_TIME.to_string(), trip_time_config)])
    }

    #[test]
    fn test_get_current_time_basic_composition() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test basic composition of start_time + trip_time
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 08:30:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(3600.0); // 1 hour
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        let expected = NaiveDateTime::parse_from_str("2023-06-15 09:30:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse expected datetime");

        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_current_time_fractional_seconds() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test with fractional seconds to verify nanosecond precision
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 08:30:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(1800.5); // 30 minutes and 500ms
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        let expected = start_datetime + chrono::Duration::new(1800, 500_000_000).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_current_time_midnight_wrapping() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test wrapping over midnight
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 23:30:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(3600.0); // 1 hour - should wrap to next day
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        let expected = NaiveDateTime::parse_from_str("2023-06-16 00:30:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse expected datetime");

        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_current_time_zero_trip_time() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test with zero trip time - should return start time unchanged
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 14:45:30", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(0.0);
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        assert_eq!(result, start_datetime);
    }

    #[test]
    fn test_get_current_time_different_time_units() {
        use chrono::NaiveDateTime;
        use routee_compass_core::model::unit::TimeUnit;
        use uom::si::time::{hour, minute};

        // Test with different TimeUnit configurations
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 10:00:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");

        // Test with minute units
        let state_model_minutes = mock_state_model(Some(TimeUnit::Minutes));
        let trip_time_minutes = Time::new::<minute>(30.0); // 30 minutes
        let state_minutes = mock_state(trip_time_minutes, &state_model_minutes);

        let result_minutes =
            super::get_current_time(&start_datetime, &state_minutes, &state_model_minutes)
                .expect("get_current_time should succeed with minutes");

        // Test with hour units
        let state_model_hours = mock_state_model(Some(TimeUnit::Hours));
        let trip_time_hours = Time::new::<hour>(0.5); // 0.5 hours = 30 minutes
        let state_hours = mock_state(trip_time_hours, &state_model_hours);

        let result_hours =
            super::get_current_time(&start_datetime, &state_hours, &state_model_hours)
                .expect("get_current_time should succeed with hours");

        let expected = NaiveDateTime::parse_from_str("2023-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse expected datetime");

        // Both should produce the same result
        assert_eq!(result_minutes, expected);
        assert_eq!(result_hours, expected);
        assert_eq!(result_minutes, result_hours);
    }

    #[test]
    fn test_get_current_time_large_trip_times() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test with large trip times (multi-day journeys)
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(259200.0); // 3 days in seconds
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        let expected = NaiveDateTime::parse_from_str("2023-06-18 12:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse expected datetime");

        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_current_time_precise_fractional_composition() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test precise fractional second handling
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 15:20:10", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);
        let trip_time = Time::new::<second>(125.123456789); // 2 minutes, 5.123456789 seconds
        let state = mock_state(trip_time, &state_model);

        let result = super::get_current_time(&start_datetime, &state, &state_model)
            .expect("get_current_time should succeed");

        // Expected: 15:20:10 + 125.123456789s = 15:22:15.123456789
        // chrono handles nanosecond precision
        let expected_seconds = 125i64;
        let expected_nanos = 123_456_789u32;
        let expected =
            start_datetime + chrono::Duration::new(expected_seconds, expected_nanos).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_current_time_various_start_times() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test with various start times to ensure consistent behavior
        let test_cases = vec![
            ("2023-01-01 00:00:00", 3661.0, "2023-01-01 01:01:01"), // New Year start
            ("2023-12-31 23:59:59", 1.0, "2024-01-01 00:00:00"),    // Year boundary
            ("2023-02-28 23:30:00", 1800.0, "2023-03-01 00:00:00"), // Month boundary (non-leap year)
            ("2024-02-28 23:30:00", 1800.0, "2024-02-29 00:00:00"), // Leap year boundary
        ];

        for (start_str, trip_seconds, expected_str) in test_cases {
            let start_datetime = NaiveDateTime::parse_from_str(start_str, "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse start datetime");
            let expected = NaiveDateTime::parse_from_str(expected_str, "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse expected datetime");

            let state_model = mock_state_model(None);
            let trip_time = Time::new::<second>(trip_seconds);
            let state = mock_state(trip_time, &state_model);

            let result = super::get_current_time(&start_datetime, &state, &state_model).expect(
                &format!("get_current_time should succeed for start: {}", start_str),
            );

            assert_eq!(
                result, expected,
                "Failed for start: {}, trip_seconds: {}, expected: {}",
                start_str, trip_seconds, expected_str
            );
        }
    }

    #[test]
    fn test_get_current_time_error_cases() {
        use chrono::NaiveDateTime;
        use uom::si::time::second;

        // Test error case: invalid duration construction
        let start_datetime =
            NaiveDateTime::parse_from_str("2023-06-15 12:00:00", "%Y-%m-%d %H:%M:%S")
                .expect("Failed to parse test datetime");
        let state_model = mock_state_model(None);

        // Test with negative time (should be caught by Duration::new if invalid)
        let trip_time = Time::new::<second>(-1.0);
        let state = mock_state(trip_time, &state_model);

        // This might succeed or fail depending on chrono's handling of negative durations
        // The behavior should be consistent
        let result = super::get_current_time(&start_datetime, &state, &state_model);

        // For negative values, we expect either success (if chrono handles it) or a specific error
        match result {
            Ok(_) => {
                // If it succeeds, the result should be before the start time
                assert!(result.unwrap() < start_datetime);
            }
            Err(e) => {
                // Should be a specific error about duration or datetime construction
                assert!(matches!(e, 
                    routee_compass_core::model::traversal::TraversalModelError::TraversalModelFailure(_) |
                    routee_compass_core::model::traversal::TraversalModelError::InternalError(_)
                ));
            }
        }
    }
}
