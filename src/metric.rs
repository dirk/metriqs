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
    Count(Id, i32),
    Gauge(Id, i32),
    Histogram(Id, i32),
}

impl Into<(Id, i32)> for CollectedMetric {
    fn into(self) -> (Id, i32) {
        use self::CollectedMetric::*;

        match self {
            Count(id, value) => (id, value),
            Gauge(id, value) => (id, value),
            Histogram(id, value) => (id, value),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AggregatedMetric {
    Count(Id, i32),
    Gauge(Id, i32),
}
