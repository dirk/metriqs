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

pub enum CollectedMetric {
    Count(Id),
    Gauge(Id),
    Histogram(Id),
}

pub enum AggregatedMetric {
    Count(Id),
    Gauge(Id),
}
