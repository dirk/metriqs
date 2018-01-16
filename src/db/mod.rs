//! In-memory metrics database used to store and aggregate metrics.

use std::cell::Cell;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use super::recv::Collector;
use super::metric::CollectedMetric;

pub struct Db {
    collection_sender: Sender<Vec<CollectedMetric>>,
    collection_receiver: Receiver<Vec<CollectedMetric>>,
    /// Collected metrics awaiting aggregation.
    collected_metrics: Mutex<Cell<Vec<CollectedMetric>>>,
    /// How long to wait before aggregating metrics. This is to account for
    /// clock variance and network lag.
    aggregation_delay: Duration,
}

impl Db {
    pub fn new() -> Db {
        let (send, recv) = channel();

        Db {
            collection_sender: send,
            collection_receiver: recv,
            collected_metrics: Mutex::new(Cell::new(Vec::new())),
            aggregation_delay: Duration::new(10, 0),
        }
    }

    pub fn collector(&self) -> Collector {
        Collector::new(self.collection_sender.clone())
    }

    pub fn recv(&self) {
        for metrics in &self.collection_receiver {
            self.collect(metrics)
        }
    }

    pub fn collect(&self, metrics: Vec<CollectedMetric>) {
        let mut cell = self.collected_metrics.lock().unwrap();
        cell.get_mut().extend(metrics);
    }
}
