use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// a row in the schedules CSV file representing, for a given route,
/// the time of departure from some source stop location and arrival at some destination
/// stop location, along some EdgeId in the RouteE Compass Graph. its unique namespace
/// is defined by it's agency_id, service_id and route_id.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleRow {
    /// edge in Compass graph this row corresponds to.
    pub edge_id: usize,
    /// the unique name of this route within this GTFS Agency
    pub route_id: String,
    /// the unique name of the service schedule attached to this Route. a Route may
    /// correspond with multiple service ids.
    pub service_id: String,
    /// the agency providing this route, if listed.
    pub agency_id: Option<String>,
    /// departure time at beginning of this edge.
    pub src_departure_time: NaiveDateTime,
    /// arrival time at end of this edge.
    pub dst_arrival_time: NaiveDateTime,
}

impl ScheduleRow {
    /// the concatenation of the agency, service, and route id.
    ///
    /// in order to allow for deconstruction of this fully-qualified name,
    /// we use a non-standard separator of multiple characters, as per the
    /// GTFS specification, ID types can contain any UTF-8 characters. see
    /// [https://gtfs.org/documentation/schedule/reference/#field-types].
    pub fn get_fully_qualified_route_id(&self) -> String {
        let agency_id = match &self.agency_id {
            Some(id) => &id,
            None => Self::EMPTY_AGENCY_PLACEHOLDER,
        };
        format!(
            "{}{}{}{}{}",
            agency_id,
            Self::FQ_ROUTE_ID_SEPARATOR,
            self.route_id,
            Self::FQ_ROUTE_ID_SEPARATOR,
            self.service_id
        )
    }

    pub const FQ_ROUTE_ID_SEPARATOR: &str = "->";

    pub const EMPTY_AGENCY_PLACEHOLDER: &str = "()";
}
