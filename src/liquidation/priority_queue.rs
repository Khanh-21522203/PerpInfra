use std::collections::BinaryHeap;
use std::cmp::Ordering;
use crate::liquidation::detector::LiquidationCandidate;

pub struct LiquidationPriorityQueue {
    heap: BinaryHeap<PriorityCandidate>,
}

impl LiquidationPriorityQueue {
    pub fn new() -> Self {
        LiquidationPriorityQueue {
            heap: BinaryHeap::new(),
        }
    }

    pub fn push(&mut self, candidate: LiquidationCandidate) {
        self.heap.push(PriorityCandidate(candidate));
    }

    pub fn pop(&mut self) -> Option<LiquidationCandidate> {
        self.heap.pop().map(|p| p.0)
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

struct PriorityCandidate(LiquidationCandidate);

impl PartialEq for PriorityCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.0.margin_ratio == other.0.margin_ratio
    }
}

impl Eq for PriorityCandidate {}

impl PartialOrd for PriorityCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower margin ratio = higher priority (reverse order)
        other.0.margin_ratio.partial_cmp(&self.0.margin_ratio)
            .unwrap_or(Ordering::Equal)
    }
}