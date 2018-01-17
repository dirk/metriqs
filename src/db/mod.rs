//! In-memory metrics database used to store and aggregate metrics.

use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

use super::recv::Collector;
use super::metric::{CollectedMetric, Id};

mod aggregate;

use self::aggregate::AggregatedMetric;

type Timeseries = (SystemTime, i32);

pub struct DbOptions {
    pub aggregation_interval: Option<Duration>,
}

impl Default for DbOptions {
    fn default() -> DbOptions {
        DbOptions {
            aggregation_interval: None,
        }
    }
}

pub struct Db {
    collection_sender: Mutex<Sender<Vec<CollectedMetric>>>,
    collection_receiver: Mutex<Receiver<Vec<CollectedMetric>>>,
    /// Collected metrics awaiting aggregation.
    collected_metrics: Mutex<Cell<Vec<CollectedMetric>>>,
    aggregation_interval: Duration,
    aggregation_subscribers: Mutex<Cell<Vec<Sender<Arc<Vec<AggregatedMetric>>>>>>,
    aggregated_metrics: Option<Mutex<Cell<HashMap<AggregatedKey, Vec<Timeseries>>>>>,
}

impl Db {
    pub fn new(options: DbOptions) -> Db {
        let aggregation_interval = options.aggregation_interval.unwrap_or_else(|| Duration::new(10, 0));

        let (send, recv) = channel();

        Db {
            collection_sender: Mutex::new(send),
            collection_receiver: Mutex::new(recv),
            collected_metrics: Mutex::new(Cell::new(vec![])),
            aggregation_interval,
            aggregation_subscribers: Mutex::new(Cell::new(vec![])),
            aggregated_metrics: Some(Mutex::new(Cell::new(HashMap::new()))),
        }
    }

    pub fn collector(&self) -> Collector {
        let sender = {
            self.collection_sender.lock().unwrap().clone()
        };
        Collector::new(sender)
    }

    /// Blocking loop to receive metrics from `Collector`s. This acquires a
    /// permanent lock on `self.collection_receiver`.
    pub fn sync_recv(&self) {
        let receiver = self.collection_receiver.lock().unwrap();
        for metrics in receiver.iter() {
            self.collect(metrics)
        }
    }

    /// Blocking loop to aggregate collected metrics.
    pub fn sync_aggregate(&self) {
        loop {
            self.aggregate();

            thread::sleep(self.aggregation_interval);
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

        if let Some(ref mutex) = self.aggregated_metrics {
            let mut cell = mutex.lock().unwrap();
            let aggregated_metrics = cell.get_mut();
            for metric in &aggregated {
                let (key, timeseries) = metric.into();
                let values = aggregated_metrics.entry(key).or_insert_with(|| vec![]);
                values.push(timeseries)
            }
        }

        let mut cell = self.aggregation_subscribers.lock().unwrap();
        let subscribers = cell.get_mut();
        let ptr = Arc::new(aggregated);
        for subscriber in subscribers {
            let _ = subscriber.send(ptr.clone());
        }
    }

    pub fn aggregation_subscribe(&self) -> Receiver<Arc<Vec<AggregatedMetric>>> {
        let (send, recv) = channel();

        let mut cell = self.aggregation_subscribers.lock().unwrap();
        let subscribers = cell.get_mut();
        subscribers.push(send);

        recv
    }
}

impl fmt::Debug for Db {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Db")
            .finish()
    }
}

/// Our timeseries "database" of aggregated metrics is keyed by the metric's
/// identifier (name and dimensions).
#[derive(Eq, Hash, PartialEq)]
enum AggregatedKey {
    Count(Id),
    Gauge(Id),
}

impl<'a> Into<(AggregatedKey, (SystemTime, i32))> for &'a AggregatedMetric {
    /// Convert an aggregated metric into a key and value for storage in the
    /// database's key-value store.
    fn into(self) -> (AggregatedKey, (SystemTime, i32)) {
        use self::AggregatedMetric::*;

        match self {
            &Count(time, ref id, value) => (AggregatedKey::Count(id.to_owned()), (time, value)),
            &Gauge(time, ref id, value) => (AggregatedKey::Gauge(id.to_owned()), (time, value)),
        }
    }
}
