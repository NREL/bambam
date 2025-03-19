use std::{cmp::Ordering, collections::BinaryHeap};

use crate::model::osm::graph::OsmNodeId;
use itertools::Itertools;
use kdam::tqdm;
use routee_compass_core::util::priority_queue::InternalPriorityQueue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ComponentFilter {
    #[default]
    Largest,
    TopK(usize),
    LeastK(usize),
}

impl std::fmt::Display for ComponentFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentFilter::Largest => write!(f, "largest"),
            ComponentFilter::TopK(k) => write!(f, "top-{}", k),
            ComponentFilter::LeastK(k) => write!(f, "least-{}", k),
        }
    }
}

impl ComponentFilter {
    /// filters the resulting connected node components.
    pub fn assign_components(&self, components: &Vec<Vec<OsmNodeId>>) -> Vec<Vec<OsmNodeId>> {
        use ComponentFilter as CF;
        let k = match self {
            CF::Largest => 1,
            CF::TopK(k) => *k,
            CF::LeastK(k) => *k,
        };
        let mut heap: BinaryHeap<FilterQueueElement> = BinaryHeap::with_capacity(k);
        let iter = tqdm!(
            components.iter().enumerate(),
            desc = format!("assign components using '{}' component filter", self),
            total = components.len()
        );
        for (idx, c) in iter {
            let element = match self {
                CF::Largest => FilterQueueElement::largest(c),
                CF::TopK(_) => FilterQueueElement::largest(c),
                CF::LeastK(_) => FilterQueueElement::smallest(c),
            };
            heap.push(element);
            if heap.len() > k {
                let _ = heap.pop();
            }
        }

        let out_components: Vec<Vec<OsmNodeId>> =
            heap.into_iter().map(|e| e.component.clone()).collect_vec();

        out_components
    }
}

#[derive(Clone, Eq, PartialEq)]
struct FilterQueueElement<'a> {
    component: &'a Vec<OsmNodeId>,
    ord: i64,
}

impl<'a> FilterQueueElement<'a> {
    /// for largest filtering, we want the smallest values to bubble to the top.
    pub fn largest(component: &'a Vec<OsmNodeId>) -> FilterQueueElement<'a> {
        FilterQueueElement {
            component,
            ord: -(component.len() as i64),
        }
    }
    /// for smallest filtering, we want the largest values to bubble to the top.
    pub fn smallest(component: &'a Vec<OsmNodeId>) -> FilterQueueElement<'a> {
        FilterQueueElement {
            component,
            ord: component.len() as i64,
        }
    }
}

impl Ord for FilterQueueElement<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ord.cmp(&other.ord)
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for FilterQueueElement<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
