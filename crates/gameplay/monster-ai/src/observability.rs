//! Low-cardinality AI counters and sampled budget-breach diagnostics.

use oteryn_error_codes::metrics::{Counter, Registry};
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::sync::{Arc, OnceLock};

fn evaluation_total() -> &'static Counter {
    static COUNTER: OnceLock<Arc<Counter>> = OnceLock::new();
    COUNTER.get_or_init(|| Registry::global().counter("monster_ai_evaluation_total", &[]))
}

fn evaluation_units_total() -> &'static Counter {
    static COUNTER: OnceLock<Arc<Counter>> = OnceLock::new();
    COUNTER.get_or_init(|| Registry::global().counter("monster_ai_evaluation_units_total", &[]))
}

fn budget_exceeded_total() -> &'static Counter {
    static COUNTER: OnceLock<Arc<Counter>> = OnceLock::new();
    COUNTER.get_or_init(|| Registry::global().counter("monster_ai_budget_exceeded_total", &[]))
}

/// Record one AI evaluation and, when refused by its budget, a sampled warning.
pub fn record_ai_evaluation(units: usize, budget: usize, over_budget: bool) {
    evaluation_total().increment();
    evaluation_units_total().add(u64::try_from(units).unwrap_or(u64::MAX));
    if !over_budget {
        return;
    }
    let exceeded = budget_exceeded_total();
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
        let start = budget_exceeded_total().get();
        let lines = capture(|| {
            record_ai_evaluation(129, 128, true);
            record_ai_evaluation(130, 128, true);
            record_ai_evaluation(131, 128, true);
        });
        let expected = (1..=3)
            .filter(|offset| start.saturating_add(*offset).is_power_of_two())
            .count();
        assert_eq!(budget_exceeded_total().get(), start + 3);
        assert_eq!(lines.len(), expected);
        assert!(
            lines
                .iter()
                .all(|line| line.contains("module=monster_ai event=ai_budget_exceeded"))
        );
        assert!(lines.iter().all(|line| !line.contains("player")));
    }
}
