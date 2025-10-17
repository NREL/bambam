/// the concatenation of the edge list, agency, service, and route id.
///
/// names are cleaned of commas for CSV compatibility.
///
/// in order to allow for deconstruction of this fully-qualified name,
/// we use a non-standard separator of multiple characters, as per the
/// GTFS specification, ID types can contain any UTF-8 characters. see
/// [https://gtfs.org/documentation/schedule/reference/#field-types].
pub fn get_fully_qualified_route_id(
    agency_id: Option<&str>,
    route_id: &str,
    service_id: &str,
    edge_list_id: usize,
) -> String {
    let agency_id = match &agency_id {
        Some(id) => id,
        None => EMPTY_AGENCY_PLACEHOLDER,
    };
    let name = format!(
        "{}{}{}{}{}{}{}",
        edge_list_id,
        FQ_ROUTE_ID_SEPARATOR,
        agency_id,
        FQ_ROUTE_ID_SEPARATOR,
        route_id,
        FQ_ROUTE_ID_SEPARATOR,
        service_id
    );
    let name_cleaned = name.replace(",", "_");
    name_cleaned
}

pub const FQ_METADATA_FIELDNAME: &str = "fq_route_ids";

pub const FQ_ROUTE_ID_SEPARATOR: &str = "->";

pub const EMPTY_AGENCY_PLACEHOLDER: &str = "()";
