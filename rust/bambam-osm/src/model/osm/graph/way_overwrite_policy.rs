
/// internal enumeration used to disambiguate graph update methods associated with
/// the adjacency list.
pub enum WayOverwritePolicy {
    /// simply append a new way onto the multiedges on this relation
    Append,
    /// way update: fail if there is no previously-existing way on this relation/index
    UpdateAtIndex { index: usize },
    /// overwrite the set of multiedges on this relation
    Replace,
    // /// way replacement: if a previously-existing way exists, use the OSM Highway
    // /// tag as a tie-breaker to determine which way to keep.
    // CompareExistingHighwayTag,
}
