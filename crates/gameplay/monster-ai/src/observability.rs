//! Low-cardinality AI counters and sampled budget-breach diagnostics.

use oteryn_error_codes::metrics::Registry;
use oteryn_error_codes::{FieldValue, Level, Line, emit};

/// Record one AI evaluation and, when refused by its budget, a sampled warning.
pub fn record_ai_evaluation(units: usize, budget: usize, over_budget: bool) {
    let metrics = Registry::global();
    metrics
        .counter("monster_ai_evaluation_total", &[])
        .increment();
    metrics
        .counter("monster_ai_evaluation_units_total", &[])
        .add(u64::try_from(units).unwrap_or(u64::MAX));
    if !over_budget {
        return;
    }
    let exceeded = metrics.counter("monster_ai_budget_exceeded_total", &[]);
    exceeded.increment();
    let count = exceeded.get();
    // First occurrence is immediately visible; powers of two retain a bounded logarithmic sample.
    if count.is_power_of_two() {
        emit(
            "oteryn-game-server",
            &Line::new(Level::Warn, "monster_ai", "ai_budget_exceeded")
                .field(
                    "units",
                    FieldValue::U64(u64::try_from(units).unwrap_or(u64::MAX)),
                )
                .field(
                    "budget",
                    FieldValue::U64(u64::try_from(budget).unwrap_or(u64::MAX)),
                )
                .field("count", FieldValue::U64(count)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_error_codes::capture;

    #[test]
    fn budget_breaches_are_counted_and_power_of_two_sampled() {
        let lines = capture(|| {
            record_ai_evaluation(129, 128, true);
            record_ai_evaluation(130, 128, true);
            record_ai_evaluation(131, 128, true);
        });
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("module=monster_ai event=ai_budget_exceeded"));
        assert!(lines.iter().all(|line| !line.contains("player")));
    }
}
