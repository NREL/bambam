use super::{calendar_date_policy::CalendarDatePolicy, schedule_error::ScheduleError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// a broker that hands out calendar policies based on the agency identifier.
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename = "snake_case")]
pub enum CalendarAssignmentPolicy {
    Global {
        policy: CalendarDatePolicy,
    },
    ByAgency {
        policies: HashMap<String, CalendarDatePolicy>,
    },
    GlobalWithExceptions {
        global_policy: CalendarDatePolicy,
        exceptions: HashMap<String, CalendarDatePolicy>,
    },
}

impl CalendarAssignmentPolicy {
    pub fn get_policy(&self, agency_id: &String) -> Result<&CalendarDatePolicy, ScheduleError> {
        match self {
            CalendarAssignmentPolicy::Global { policy } => Ok(policy),
            CalendarAssignmentPolicy::ByAgency { policies } => policies
                .get(agency_id)
                .ok_or_else(|| ScheduleError::UnknownAgencyId(agency_id.clone())),
            CalendarAssignmentPolicy::GlobalWithExceptions {
                global_policy,
                exceptions,
            } => match exceptions.get(agency_id) {
                Some(policy) => Ok(policy),
                None => Ok(global_policy),
            },
        }
    }
}
