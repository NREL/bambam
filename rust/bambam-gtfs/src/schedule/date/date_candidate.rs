use gtfs_structures::CalendarDate;

// Create a wrapper type for BinaryHeap ordering
#[derive(Debug)]
pub struct DateCandidate(pub u64, pub CalendarDate);

impl Ord for DateCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse the ordering to make BinaryHeap work as min-heap for distance
        other
            .0
            .cmp(&self.0)
            .then_with(|| other.1.date.cmp(&self.1.date))
    }
}

impl PartialOrd for DateCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DateCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
            && self.1.date == other.1.date
            && self.1.exception_type == other.1.exception_type
            && self.1.service_id == other.1.service_id
    }
}

impl Eq for DateCandidate {}
