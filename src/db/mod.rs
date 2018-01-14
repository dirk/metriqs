//! In-memory metrics database used to store and aggregate metrics.

use std::collections::VecDeque;

use super::metric::CollectedMetric;

pub struct Db {
    buf: VecDeque<CollectedMetric>,
}
