//! Channel-owner work-item counters and slow-item diagnostics.

use oteryn_error_codes::metrics::Registry;
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::time::Duration;

/// Record one completed Channel work item. `kind` must be a bounded operation token.
pub fn record_channel_work_item(kind: &'static str, elapsed: Duration, slow_after: Duration) {
    let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let metrics = Registry::global();
    metrics
        .counter("channel_work_item_total", &[("kind", kind)])
        .increment();
    metrics
        .histogram("channel_work_item_ms", &[("kind", kind)])
        .observe(elapsed_ms);
    if elapsed > slow_after {
        metrics
            .counter("channel_work_item_overrun_total", &[("kind", kind)])
            .increment();
        emit(
            "oteryn-game-server",
            &Line::new(Level::Warn, "channel", "work_item_overrun")
                .field("kind", FieldValue::Token(kind))
                .field("ms", FieldValue::U64(elapsed_ms))
                .field(
                    "threshold_ms",
                    FieldValue::U64(u64::try_from(slow_after.as_millis()).unwrap_or(u64::MAX)),
                ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_error_codes::capture;

    #[test]
    fn only_slow_work_items_emit_a_warning() {
        let lines = capture(|| {
            record_channel_work_item(
                "movement",
                Duration::from_millis(50),
                Duration::from_millis(50),
            );
            record_channel_work_item("cast", Duration::from_millis(51), Duration::from_millis(50));
        });
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("module=channel event=work_item_overrun"));
        assert!(lines[0].contains("kind=cast ms=51 threshold_ms=50"));
    }
}
