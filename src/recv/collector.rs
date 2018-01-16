use std::collections::VecDeque;

use super::super::metric::CollectedMetric;

pub struct Collector {
    buf: VecDeque<CollectedMetric>,
}

impl Collector {
    pub fn new() -> Collector {
        Collector {
            buf: VecDeque::new(),
        }
    }

    pub fn push(&mut self, metric: CollectedMetric) {
        self.buf.push_back(metric)
    }
}
