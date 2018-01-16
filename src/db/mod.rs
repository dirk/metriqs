//! In-memory metrics database used to store and aggregate metrics.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, SystemTime};

use string_cache::DefaultAtom as Atom;

use super::recv::Collector;
use super::metric::{AggregatedMetric, CollectedMetric, Id};

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

    pub fn aggregate(&self) {
        #[derive(Eq, Hash, PartialEq)]
        enum Group {
            Count(Id),
            Gauge(Id),
            Histogram(Id),
        }

        let now = SystemTime::now();

        // Get all the collected metrics; replaces it with an empty `Vec`
        // before releasing the lock (so that other threads can continue
        // adding metrics).
        let collected_metrics = {
            let cell = self.collected_metrics.lock().unwrap();
            cell.replace(Vec::new())
        };

        let mut grouped = HashMap::<Group, Vec<i32>>::new();
        for metric in collected_metrics {
            let (group, value) = match metric {
                CollectedMetric::Count(id, value)     => (Group::Count(id), value),
                CollectedMetric::Gauge(id, value)     => (Group::Gauge(id), value),
                CollectedMetric::Histogram(id, value) => (Group::Histogram(id), value),
            };
            let values = grouped.entry(group).or_insert_with(|| vec![]);
            values.push(value)
        }

        let mut aggregated = Vec::<AggregatedMetric>::new();
        for (group, values) in grouped.into_iter() {
            use self::AggregatedMetric::*;
            
            match group {
                Group::Count(id) => {
                    let count = values.iter().fold(0, |memo, value| memo + value);
                    aggregated.push(Count(id, count))
                },
                Group::Gauge(id) => {
                    let max = values.iter().max().unwrap_or(&0);
                    aggregated.push(Gauge(id, *max))
                },
                Group::Histogram(id) => {
                    let histogram = Histogram::from(&values);

                    aggregated.push(Gauge(suffix_id(&id, ".min"), histogram.min));
                    aggregated.push(Gauge(suffix_id(&id, ".max"), histogram.max));
                    aggregated.push(Gauge(suffix_id(&id, ".median"), histogram.median));
                    aggregated.push(Gauge(suffix_id(&id, ".avg"), histogram.average));
                    aggregated.push(Gauge(suffix_id(&id, ".95percentile"), histogram.percentile95));
                    aggregated.push(Gauge(suffix_id(&id, ".99percentile"), histogram.percentile99));

                    aggregated.push(Count(suffix_id(&id, ".count"), values.len() as i32));
                },
            }
        }

        let mut cell = self.aggregated_metrics.lock().unwrap();
        let aggregated_metrics = cell.get_mut();
        for metric in aggregated {
            let (key, value) = metric.into();
            let values = aggregated_metrics.entry(key).or_insert_with(|| vec![]);
            values.push((now, value))
        }
    }
}

fn suffix_id(id: &Id, suffix: &str) -> Id {
    let &(ref name_atom, ref dimensions) = id;
    let name: &str = &name_atom;

    (Atom::from(format!("{}{}", name, suffix)), dimensions.to_owned())
}

/// Our timeseries "database" of aggregated metrics is keyed by the metric's
/// identifier (name and dimensions).
#[derive(Eq, Hash, PartialEq)]
enum AggregatedKey {
    Count(Id),
    Gauge(Id),
}

impl Into<(AggregatedKey, i32)> for AggregatedMetric {
    /// Convert an aggregated metric into a key and value for storage in the
    /// database's key-value store.
    fn into(self) -> (AggregatedKey, i32) {
        use self::AggregatedMetric::*;

        match self {
            Count(id, value) => (AggregatedKey::Count(id), value),
            Gauge(id, value) => (AggregatedKey::Gauge(id), value),
        }
    }
}

struct Histogram {
    min: i32,
    max: i32,
    median: i32,
    average: i32,
    percentile95: i32,
    percentile99: i32,
}

impl<'a> From<&'a Vec<i32>> for Histogram {
    fn from(values: &'a Vec<i32>) -> Histogram {
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.cmp(b));

        Histogram {
            min:          *sorted.first().unwrap(),
            max:          *sorted.last().unwrap(),
            median:       sorted[sorted.len() / 2], // TODO: Improve how we calculate the median
            average:      (sorted.iter().fold(0.0, |sum, val| { sum + (*val as f64) }) / (sorted.len() as f64)) as i32,
            percentile95: sorted[(sorted.len() as f64 * 0.95) as usize],
            percentile99: sorted[(sorted.len() as f64 * 0.99) as usize],
        }
    }
}
