use bamcensus_core::model::identifier::Geoid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Grouping {
    pub geoid: Geoid,
    pub mode: String,
}

impl Grouping {
    pub fn new(geoid: Geoid, mode: String) -> Grouping {
        Grouping { geoid, mode }
    }
}
