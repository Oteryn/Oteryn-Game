//! Bounded in-process counters and fixed-bucket duration histograms.

use crate::{FieldValue, Level, Line, emit};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Maximum number of non-overflow metric series in a registry.
pub const MAX_SERIES: usize = 512;
/// Fixed finite histogram bounds in milliseconds.
pub const HISTOGRAM_BOUNDS_MS: [u64; 13] = [
    1,
    5,
    10,
    25,
    50,
    100,
    250,
    500,
    1_000,
    2_500,
    5_000,
    10_000,
    u64::MAX,
];

/// An atomic monotonically increasing counter.
#[derive(Debug, Default)]
pub struct Counter(AtomicU64);

impl Counter {
    /// Increment the counter by one.
    pub fn increment(&self) {
        self.add(1);
    }

    /// Add `value`, saturating at `u64::MAX`.
    pub fn add(&self, value: u64) {
        let _ = self
            .0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(value))
            });
    }

    /// Read the current value.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// A fixed-bucket millisecond histogram.
#[derive(Debug)]
pub struct Histogram {
    buckets: [AtomicU64; HISTOGRAM_BOUNDS_MS.len()],
    count: AtomicU64,
    sum: AtomicU64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }
}

impl Histogram {
    /// Observe one duration in milliseconds.
    pub fn observe(&self, milliseconds: u64) {
        let index = HISTOGRAM_BOUNDS_MS.partition_point(|bound| *bound < milliseconds);
        self.buckets[index].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        let _ = self
            .sum
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |sum| {
                Some(sum.saturating_add(milliseconds))
            });
    }

    /// Take a consistent-enough lock-free snapshot for diagnostics.
    #[must_use]
    pub fn snapshot(&self) -> HistogramSnapshot {
        HistogramSnapshot {
            buckets: std::array::from_fn(|index| self.buckets[index].load(Ordering::Relaxed)),
            count: self.count.load(Ordering::Relaxed),
            sum: self.sum.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time histogram summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistogramSnapshot {
    /// Per-bucket, non-cumulative counts.
    pub buckets: [u64; HISTOGRAM_BOUNDS_MS.len()],
    /// Total observations.
    pub count: u64,
    /// Saturating sum of observed milliseconds.
    pub sum: u64,
}

impl HistogramSnapshot {
    /// Approximate p50 in milliseconds from the fixed buckets.
    #[must_use]
    pub fn p50(&self) -> u64 {
        self.percentile(50)
    }
    /// Approximate p95 in milliseconds from the fixed buckets.
    #[must_use]
    pub fn p95(&self) -> u64 {
        self.percentile(95)
    }
    /// Approximate p99 in milliseconds from the fixed buckets.
    #[must_use]
    pub fn p99(&self) -> u64 {
        self.percentile(99)
    }

    fn percentile(&self, percentile: u64) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let target = self.count.saturating_mul(percentile).div_ceil(100);
        let mut cumulative = 0_u64;
        for (index, count) in self.buckets.iter().enumerate() {
            cumulative = cumulative.saturating_add(*count);
            if cumulative >= target {
                return HISTOGRAM_BOUNDS_MS[index];
            }
        }
        u64::MAX
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SeriesKey {
    name: String,
    labels: Vec<(String, String)>,
    kind: SeriesKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SeriesKind {
    Counter,
    Histogram,
}

#[derive(Debug)]
enum Series {
    Counter(Arc<Counter>),
    Histogram(Arc<Histogram>),
}

/// A process metric registry bounded to 512 named and labelled series.
#[derive(Debug, Default)]
pub struct Registry {
    series: Mutex<BTreeMap<SeriesKey, Series>>,
    overflow: Arc<Counter>,
    overflow_series: Arc<Counter>,
}

impl Registry {
    /// Return the process-global registry.
    #[must_use]
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<Registry> = OnceLock::new();
        REGISTRY.get_or_init(Self::default)
    }

    /// Get or register a counter with at most two token labels.
    #[must_use]
    pub fn counter(&self, name: &str, labels: &[(&str, &str)]) -> Arc<Counter> {
        let key = series_key(name, labels, SeriesKind::Counter);
        let mut series = self
            .series
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(Series::Counter(counter)) = series.get(&key) {
            return Arc::clone(counter);
        }
        if series.len() >= MAX_SERIES || labels.len() > 2 {
            self.overflow.increment();
            return Arc::clone(&self.overflow_series);
        }
        let counter = Arc::new(Counter::default());
        series.insert(key, Series::Counter(Arc::clone(&counter)));
        counter
    }

    /// Get or register a histogram with at most two token labels.
    #[must_use]
    pub fn histogram(&self, name: &str, labels: &[(&str, &str)]) -> Arc<Histogram> {
        let key = series_key(name, labels, SeriesKind::Histogram);
        let mut series = self
            .series
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(Series::Histogram(histogram)) = series.get(&key) {
            return Arc::clone(histogram);
        }
        if series.len() >= MAX_SERIES || labels.len() > 2 {
            self.overflow.increment();
            return Arc::new(Histogram::default());
        }
        let histogram = Arc::new(Histogram::default());
        series.insert(key, Series::Histogram(Arc::clone(&histogram)));
        histogram
    }

    /// Number of rejected series registration attempts.
    #[must_use]
    pub fn overflow_count(&self) -> u64 {
        self.overflow.get()
    }

    /// Render the registry in Prometheus text exposition format.
    #[must_use]
    pub fn render_prometheus(&self) -> String {
        let series = self
            .series
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut output = String::new();
        for (key, value) in series.iter() {
            match value {
                Series::Counter(counter) => {
                    let _ = writeln!(
                        output,
                        "{}{} {}",
                        metric_token(&key.name),
                        prometheus_labels(&key.labels),
                        counter.get()
                    );
                }
                Series::Histogram(histogram) => {
                    render_histogram(&mut output, key, &histogram.snapshot())
                }
            }
        }
        let _ = writeln!(
            output,
            "oteryn_metrics_overflow_total {}",
            self.overflow.get()
        );
        output
    }

    /// Build the periodic stats line with bounded registry totals.
    #[must_use]
    pub fn stats_line(&self) -> Line<'static> {
        let series = self
            .series
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let top = series
            .iter()
            .map(|(key, series)| {
                let value = match series {
                    Series::Counter(counter) => counter.get(),
                    Series::Histogram(histogram) => histogram.snapshot().count,
                };
                (key, value)
            })
            .max_by_key(|(_, value)| *value);
        let mut line = Line::new(Level::Info, "stats", "stats")
            .field("series", FieldValue::U64(series.len() as u64))
            .field("overflow", FieldValue::U64(self.overflow.get()));
        if let Some((key, value)) = top {
            let mut top_name = key.name.clone();
            for (label, value) in &key.labels {
                top_name.push('_');
                top_name.push_str(label);
                top_name.push('_');
                top_name.push_str(value);
            }
            line = line
                .field("top", FieldValue::Owned(top_name))
                .field("top_value", FieldValue::U64(value));
        }
        line
    }
}

/// Render the process-global registry in Prometheus text exposition format.
#[must_use]
pub fn render_prometheus() -> String {
    Registry::global().render_prometheus()
}

/// Build the process-global periodic stats line.
#[must_use]
pub fn stats_line() -> Line<'static> {
    Registry::global().stats_line()
}

/// Emit one process-global periodic stats line.
pub fn emit_stats(process: &str) {
    emit(process, &stats_line());
}

fn series_key(name: &str, labels: &[(&str, &str)], kind: SeriesKind) -> SeriesKey {
    SeriesKey {
        name: metric_token(name),
        labels: labels
            .iter()
            .take(2)
            .map(|(key, value)| (metric_token(key), metric_token(value)))
            .collect(),
        kind,
    }
}

fn metric_token(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .take(128)
        .collect();
    if value.is_empty() {
        "_".to_owned()
    } else {
        value
    }
}

fn prometheus_labels(labels: &[(String, String)]) -> String {
    if labels.is_empty() {
        return String::new();
    }
    let body = labels
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{body}}}")
}

fn render_histogram(output: &mut String, key: &SeriesKey, snapshot: &HistogramSnapshot) {
    let name = metric_token(&key.name);
    let mut cumulative = 0_u64;
    for (index, count) in snapshot.buckets.iter().enumerate() {
        cumulative = cumulative.saturating_add(*count);
        let bound = if HISTOGRAM_BOUNDS_MS[index] == u64::MAX {
            "+Inf".to_owned()
        } else {
            HISTOGRAM_BOUNDS_MS[index].to_string()
        };
        let mut labels = key.labels.clone();
        labels.push(("le".to_owned(), bound));
        let _ = writeln!(
            output,
            "{name}_bucket{} {cumulative}",
            prometheus_labels(&labels)
        );
    }
    let labels = prometheus_labels(&key.labels);
    let _ = writeln!(output, "{name}_sum{labels} {}", snapshot.sum);
    let _ = writeln!(output, "{name}_count{labels} {}", snapshot.count);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn histogram_reports_bucket_percentiles_count_and_sum() {
        let histogram = Histogram::default();
        for value in [1, 2, 5, 20, 100, 1_000, 20_000] {
            histogram.observe(value);
        }
        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count, 7);
        assert_eq!(snapshot.sum, 21_128);
        assert_eq!(snapshot.p50(), 25);
        assert_eq!(snapshot.p95(), u64::MAX);
        assert_eq!(snapshot.p99(), u64::MAX);
    }

    #[test]
    fn cardinality_overflow_is_bounded_and_counted() {
        let registry = Registry::default();
        for index in 0..MAX_SERIES + 3 {
            registry
                .counter(&format!("counter_{index}"), &[])
                .increment();
        }
        assert_eq!(registry.series.lock().unwrap().len(), MAX_SERIES);
        assert_eq!(registry.overflow_count(), 3);
        assert!(
            registry
                .render_prometheus()
                .contains("oteryn_metrics_overflow_total 3")
        );
    }

    #[test]
    fn prometheus_histogram_contains_cumulative_buckets() {
        let registry = Registry::default();
        let histogram = registry.histogram("request_ms", &[("route", "login")]);
        histogram.observe(4);
        histogram.observe(12);
        let rendered = registry.render_prometheus();
        assert!(rendered.contains("request_ms_bucket{route=\"login\",le=\"5\"} 1"));
        assert!(rendered.contains("request_ms_count{route=\"login\"} 2"));
        let stats = registry.stats_line().render("p", 1);
        assert!(stats.contains("top=request_ms_route_login top_value=2"));
    }
}
