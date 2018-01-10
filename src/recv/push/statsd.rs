use string_cache::DefaultAtom as Atom;

pub enum StatsdMetric {
    /// Name, value, sample rate
    Counter(Atom, f64, Option<f64>),
    /// Name, value, sample rate
    Timer(Atom, f64, Option<f64>),
    /// Name, value
    Gauge(Atom, f64),
}
