use std::sync::mpsc::Sender;

use super::super::metric::CollectedMetric;

pub struct Collector {
    sender: Sender<Vec<CollectedMetric>>,
}

impl Collector {
    pub fn new(sender: Sender<Vec<CollectedMetric>>) -> Collector {
        Collector {
            sender: sender,
        }
    }

    pub fn push(&self, metrics: Vec<CollectedMetric>) {
        let _ = self.sender.send(metrics);
    }
}
