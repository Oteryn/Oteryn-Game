//! Bounded world-tick telemetry. The owner supplies thresholds from node configuration.

use oteryn_error_codes::metrics::Registry;
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::time::{Duration, Instant};

/// Aggregates tick overruns so a busy world writes at most one warning per report interval.
#[derive(Debug)]
pub struct WorldTickTelemetry {
    budget: Duration,
    report_interval: Duration,
    window_started: Instant,
    breaches: u64,
}

impl WorldTickTelemetry {
    #[must_use]
    pub const fn new(budget: Duration, report_interval: Duration, now: Instant) -> Self {
        Self {
            budget,
            report_interval,
            window_started: now,
            breaches: 0,
        }
    }

    /// Count and time one tick, emitting an aggregate warning when its reporting window closes.
    pub fn record(&mut self, elapsed: Duration, now: Instant) {
        let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let metrics = Registry::global();
        metrics.counter("world_tick_total", &[]).increment();
        metrics.histogram("world_tick_ms", &[]).observe(elapsed_ms);
        if elapsed > self.budget {
            self.breaches = self.breaches.saturating_add(1);
            metrics.counter("world_tick_overrun_total", &[]).increment();
        }
        if now.duration_since(self.window_started) >= self.report_interval {
            if self.breaches != 0 {
                emit(
                    "oteryn-game-server",
                    &Line::new(Level::Warn, "world", "tick_overrun")
                        .field("breaches", FieldValue::U64(self.breaches))
                        .field(
                            "window_ms",
                            FieldValue::U64(
                                u64::try_from(self.report_interval.as_millis()).unwrap_or(u64::MAX),
                            ),
                        ),
                );
            }
            self.breaches = 0;
            self.window_started = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_error_codes::capture;

    #[test]
    fn overruns_are_counted_and_aggregated_once_per_window() {
        let start = Instant::now();
        let mut telemetry =
            WorldTickTelemetry::new(Duration::from_millis(20), Duration::from_secs(10), start);
        let lines = capture(|| {
            telemetry.record(Duration::from_millis(21), start + Duration::from_secs(1));
            telemetry.record(Duration::from_millis(30), start + Duration::from_secs(10));
        });
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("level=warn module=world event=tick_overrun"));
        assert!(lines[0].contains("breaches=2 window_ms=10000"));
    }
}
