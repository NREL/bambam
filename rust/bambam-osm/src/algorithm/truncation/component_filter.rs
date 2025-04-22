use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{BinaryHeap, HashSet},
};

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
    KeepAll,
}

impl std::fmt::Display for ComponentFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentFilter::Largest => write!(f, "largest"),
            ComponentFilter::TopK(k) => write!(f, "top-{}", k),
            ComponentFilter::LeastK(k) => write!(f, "least-{}", k),
            ComponentFilter::KeepAll => write!(f, "keep all"),
        }
    }
}

impl ComponentFilter {
    /// filters the resulting connected node components.
    pub fn assign_components(&self, components: Vec<Vec<OsmNodeId>>) -> Vec<Vec<OsmNodeId>> {
        use ComponentFilter as CF;
        let k = match self {
            CF::Largest => 1,
            CF::TopK(k) => *k,
            CF::LeastK(k) => *k,
            CF::KeepAll => return components,
        };
        let mut heap: BinaryHeap<FilterQueueElement> = BinaryHeap::with_capacity(k);
        let iter = tqdm!(
            components.iter().enumerate(),
            desc = format!("assign components using '{}' component filter", self),
            total = components.len()
        );

        for (idx, c) in iter {
            let element = match self {
                CF::Largest => FilterQueueElement::largest(c, idx),
                CF::TopK(_) => FilterQueueElement::largest(c, idx),
                CF::LeastK(_) => FilterQueueElement::smallest(c, idx),
                CF::KeepAll => panic!("runtime error, KeepAll variant should not reach this point"),
            };
            heap.push(element);
            if heap.len() > k {
                let _ = heap.pop();
            }
        }
        let keep_indices: HashSet<usize> = heap.iter().map(|fqe| fqe.index).collect();
        if keep_indices.len() == components.len() {
            return components;
        }

        let mut out_components: Vec<Vec<OsmNodeId>> = Vec::with_capacity(k);
        for (idx, component) in components.into_iter().enumerate() {
            if keep_indices.contains(&idx) {
                out_components.push(component);
            }
        }

        out_components
    }
}

#[derive(Clone, Eq, PartialEq)]
struct FilterQueueElement {
    // component: &'a Vec<OsmNodeId>,
    ord: i64,
    index: usize,
}

impl FilterQueueElement {
    /// for largest filtering, we want the smallest values to bubble to the top.
    pub fn largest(component: &[OsmNodeId], index: usize) -> FilterQueueElement {
        FilterQueueElement {
            // component,
            ord: -(component.len() as i64),
            index,
        }
    }
    /// for smallest filtering, we want the largest values to bubble to the top.
    pub fn smallest(component: &[OsmNodeId], index: usize) -> FilterQueueElement {
        FilterQueueElement {
            // component,
            ord: component.len() as i64,
            index,
        }
    }
}

impl Ord for FilterQueueElement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ord.cmp(&other.ord)
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for FilterQueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
