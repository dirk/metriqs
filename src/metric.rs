use std::time::{SystemTime};

use string_cache::DefaultAtom as Atom;

// Aliases:
//   Count:
//     StatsD: Counter
//     Datadog: Count
//     Prometheus: Counter
//   Gauge:
//     StatsD: Gauge
//     Datadog: Gauge
//     Prometheus: Gauge
//   Histogram:
//     StatsD: Timer
//     Datadog: Histogram
//     Prometheus: Histogram

pub type Dimension = (Atom, Atom);

pub type Id = (Atom, Vec<Dimension>);

#[derive(Debug, PartialEq)]
pub enum CollectedMetric {
    Count(SystemTime, Id, i32),
    Gauge(SystemTime, Id, i32),
    Histogram(SystemTime, Id, i32),
}
