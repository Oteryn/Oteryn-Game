//! AI counters, sampled budget breaches and debug decision diagnostics.

use crate::foundation::ExactActorRef;
use oteryn_error_codes::metrics::{Counter, Registry};
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::sync::{Arc, OnceLock};

fn decision_level() -> Level {
    static LEVEL: OnceLock<Level> = OnceLock::new();
    *LEVEL.get_or_init(|| {
        if std::env::var("OTERYN_QUALIFICATION_LOG_PROFILE").as_deref() == Ok("monster_ai") {
            Level::Info
        } else {
            Level::Debug
        }
    })
}

/// Whether the owner should collect the per-monster detail for a refused turn.
pub fn melee_diagnostics_enabled() -> bool {
    oteryn_error_codes::filter().allows(decision_level(), "monster_ai", false)
}

pub(super) fn record_target_decision(
    monster: ExactActorRef,
    target: Option<ExactActorRef>,
    reason: &str,
) {
    if !melee_diagnostics_enabled() {
        return;
    }
    emit(
        "oteryn-game-server",
        &Line::new(decision_level(), "monster_ai", "target_decision")
            .field("monster", FieldValue::Owned(monster_identity(monster)))
            .field(
                "target",
                target.map_or(FieldValue::Token("none"), |actor| {
                    FieldValue::Owned(format!(
                        "{}/{}",
                        actor.actor_local_id(),
                        actor.actor_local_generation()
                    ))
                }),
            )
            .field("reason", FieldValue::Token(reason)),
    );
}

fn monster_identity(actor: ExactActorRef) -> String {
    let identity = u128::from_be_bytes(actor.placement_identity());
    format!("{identity:032x}/{}", actor.actor_local_generation())
}

pub fn record_melee_outcome(monster: ExactActorRef, outcome: &str, reason: &str) {
    if !melee_diagnostics_enabled() {
        return;
    }
    emit(
        "oteryn-game-server",
        &Line::new(decision_level(), "monster_ai", "melee_drain")
            .field("monster", FieldValue::Owned(monster_identity(monster)))
            .field("outcome", FieldValue::Token(outcome))
            .field("reason", FieldValue::Token(reason)),
    );
}

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
