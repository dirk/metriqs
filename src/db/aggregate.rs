use std::collections::HashMap;
use std::iter::Iterator;
use std::time::SystemTime;

use string_cache::DefaultAtom as Atom;

use super::super::metric::{CollectedMetric, Id};

#[derive(Eq, Hash, PartialEq)]
pub enum Group {
    Count(Id),
    Gauge(Id),
    Histogram(Id),
}

type GroupedMetrics = HashMap<Group, Vec<(SystemTime, i32)>>;

/// Group metrics by their identifier.
pub fn group<T: AsRef<Vec<CollectedMetric>>>(metrics: T) -> GroupedMetrics {
    let metrics = metrics.as_ref();
    let mut grouped = GroupedMetrics::new();
    for metric in metrics.into_iter() {
        let (group, value) = match metric {
            &CollectedMetric::Count(time, ref id, value)     => (Group::Count(id.to_owned()), (time, value)),
            &CollectedMetric::Gauge(time, ref id, value)     => (Group::Gauge(id.to_owned()), (time, value)),
            &CollectedMetric::Histogram(time, ref id, value) => (Group::Histogram(id.to_owned()), (time, value)),
        };
        let values = grouped.entry(group).or_insert_with(|| vec![]);
        values.push(value)
    }
    grouped
}

pub enum AggregatedMetric {
    Count(SystemTime, Id, i32),
    Gauge(SystemTime, Id, i32),
}

pub fn aggregate(grouped: GroupedMetrics) -> Vec<AggregatedMetric> {
    let mut aggregated = Vec::<AggregatedMetric>::new();
    for (group, timeseries) in grouped.into_iter() {
        use self::AggregatedMetric::*;

        let time = match timeseries.iter().max_by(|x, y| x.0.cmp(&y.0)) {
            Some(t) => t.0,
            None => continue,
        };
        let values = timeseries.iter().map(|t| t.1).collect::<Vec<i32>>();

        match group {
            Group::Count(id) => {
                let count = values.iter().fold(0, |memo, value| memo + value);
                aggregated.push(Count(time, id, count))
            },
            Group::Gauge(id) => {
                let max = values.iter().max_by(|x, y| x.cmp(&y)).unwrap_or(&0);
                aggregated.push(Gauge(time, id, *max))
            },
            Group::Histogram(id) => {
                let histogram = Histogram::from(&values);

                aggregated.push(Gauge(time, suffix_id(&id, ".min"), histogram.min));
                aggregated.push(Gauge(time, suffix_id(&id, ".max"), histogram.max));
                aggregated.push(Gauge(time, suffix_id(&id, ".median"), histogram.median));
                aggregated.push(Gauge(time, suffix_id(&id, ".avg"), histogram.average));
                aggregated.push(Gauge(time, suffix_id(&id, ".95percentile"), histogram.percentile95));
                aggregated.push(Gauge(time, suffix_id(&id, ".99percentile"), histogram.percentile99));

                aggregated.push(Count(time, suffix_id(&id, ".count"), values.len() as i32));
            },
        }
    }
    aggregated
}

/// Add a suffix to the end of the name of a metric.
fn suffix_id<S: AsRef<str>>(id: &Id, suffix: S) -> Id {
    let &(ref name_atom, ref dimensions) = id;
    let name: &str = &name_atom;

    (Atom::from(format!("{}{}", name, suffix.as_ref())), dimensions.to_owned())
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

