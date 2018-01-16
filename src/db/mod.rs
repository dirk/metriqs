//! In-memory metrics database used to store and aggregate metrics.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, SystemTime};

use super::recv::Collector;
use super::metric::{CollectedMetric, Id};

mod aggregate;

use self::aggregate::AggregatedMetric;

type Timeseries = (SystemTime, i32);

pub struct Db {
    collection_sender: Sender<Vec<CollectedMetric>>,
    collection_receiver: Receiver<Vec<CollectedMetric>>,
    /// Collected metrics awaiting aggregation.
    collected_metrics: Mutex<Cell<Vec<CollectedMetric>>>,
    aggregation_interval: Duration,
    aggregated_metrics: Mutex<Cell<HashMap<AggregatedKey, Vec<Timeseries>>>>,
}

impl Db {
    pub fn new() -> Db {
        let (send, recv) = channel();

        Db {
            collection_sender: send,
            collection_receiver: recv,
            collected_metrics: Mutex::new(Cell::new(vec![])),
            aggregation_interval: Duration::new(10, 0),
            aggregated_metrics: Mutex::new(Cell::new(HashMap::new())),
        }
    }

    pub fn collector(&self) -> Collector {
        Collector::new(self.collection_sender.clone())
    }

    /// Blocking loop to receive metrics from `Collector`s.
    pub fn recv(&self) {
        for metrics in &self.collection_receiver {
            self.collect(metrics)
        }
    }

    pub fn collect(&self, metrics: Vec<CollectedMetric>) {
        let mut cell = self.collected_metrics.lock().unwrap();
        cell.get_mut().extend(metrics);
    }

    // TODO: Add a window option to the call.
    pub fn aggregate(&self) {
        // Get all the collected metrics; replaces it with an empty `Vec`
        // before releasing the lock so that other threads can continue
        // adding metrics.
        let collected_metrics = {
            let cell = self.collected_metrics.lock().unwrap();
            cell.replace(Vec::new())
        };

        // Convert raw metrics into groups keyed by the identifier and
        // with raw timeseries as the values.
        let grouped = aggregate::group(collected_metrics);

        // Roll up each metric.
        let aggregated = aggregate::aggregate(grouped);

        let mut cell = self.aggregated_metrics.lock().unwrap();
        let aggregated_metrics = cell.get_mut();
        for metric in aggregated {
            let (key, timeseries) = metric.into();
            let values = aggregated_metrics.entry(key).or_insert_with(|| vec![]);
            values.push(timeseries)
        }
    }
}

/// Our timeseries "database" of aggregated metrics is keyed by the metric's
/// identifier (name and dimensions).
#[derive(Eq, Hash, PartialEq)]
enum AggregatedKey {
    Count(Id),
    Gauge(Id),
}

impl Into<(AggregatedKey, (SystemTime, i32))> for AggregatedMetric {
    /// Convert an aggregated metric into a key and value for storage in the
    /// database's key-value store.
    fn into(self) -> (AggregatedKey, (SystemTime, i32)) {
        use self::AggregatedMetric::*;

        match self {
            Count(time, id, value) => (AggregatedKey::Count(id), (time, value)),
            Gauge(time, id, value) => (AggregatedKey::Gauge(id), (time, value)),
        }
    }
}
