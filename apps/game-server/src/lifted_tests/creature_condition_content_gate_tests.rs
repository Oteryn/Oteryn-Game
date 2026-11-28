//! CONDITIONS-0 §3.5 (MAGIC-COND-1): content lowers the gate families by name and
//! requires a damage schedule for Agony.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use crate::content::ProjectV2InlineEffect;
use crate::creature_condition_content::*;
use crate::foundation::condition::COND0_RL_02_MIN_TICK_INTERVAL_MS;
use crate::foundation::{ConditionDefinition, ConditionValues};
use serde_json::{Value, json};

fn drunk() -> Value {
    let rows: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/agents/evidence/monster-runtime-project-20261004/lanes/special/condition-fixtures.json"
    )))
    .unwrap();
    rows.into_iter()
        .find(|r| r["effect"]["operation"]["condition"]["condition_type"] == "drunk")
        .unwrap()
}

fn lower_as(
    kind: &str,
    edit: impl FnOnce(&mut Value),
) -> Result<ConditionDefinition, ConditionContentError> {
    let mut row = drunk();
    row["effect"]["operation"]["condition"]["condition_type"] = json!(kind);
    edit(&mut row);
    let effect: ProjectV2InlineEffect = serde_json::from_value(row["effect"].clone()).unwrap();
    lower_condition_definition(&effect, 1, None)
}

#[test]
fn gate_families_lower_with_their_fixed_duration() {
    let duration = drunk()["effect"]["operation"]["duration_ms"]
        .as_u64()
        .unwrap();
    for kind in ["muted", "pacified", "powerless"] {
        let definition = lower_as(kind, |_| {}).unwrap();
        let ConditionValues::Gate { gate, duration_ms } = definition.values() else {
            panic!("{kind} must lower to a gate family");
        };
        assert_eq!(gate.key(), kind);
        assert_eq!(u64::from(duration_ms), duration);
    }
}

#[test]
fn unknown_names_and_extra_gate_payload_fail_closed() {
    for kind in ["mute", "Pacified", ""] {
        assert_eq!(
            lower_as(kind, |_| {}).unwrap_err(),
            ConditionContentError::UnsupportedCondition
        );
    }
    assert_eq!(
        lower_as("pacified", |row| {
            row["effect"]["operation"]["condition"]["speed_formula"] =
                json!({"family": "Formula", "key": "oteryn:formula.x", "revision": "definition-r1"});
        })
        .unwrap_err(),
        ConditionContentError::UnsupportedParameters
    );
    assert_eq!(
        lower_as("muted", |row| {
            row["effect"]["operation"]["duration_ms"] = Value::Null;
        })
        .unwrap_err(),
        ConditionContentError::InvalidDefinition
    );
}

#[test]
fn agony_requires_an_exact_bounded_damage_schedule() {
    assert_eq!(
        lower_as("agony", |_| {}).unwrap_err(),
        ConditionContentError::UnsupportedParameters
    );
    let min_interval = COND0_RL_02_MIN_TICK_INTERVAL_MS;
    for (ticks, expected) in [
        (json!([]), ConditionContentError::UnsupportedDamageSchedule),
        (
            json!([{"count": 2, "interval_ms": min_interval, "amount": u32::MAX}]),
            ConditionContentError::NumericOverflow,
        ),
        (
            json!([{"count": 1, "interval_ms": min_interval - 1, "amount": 100}]),
            ConditionContentError::InvalidDefinition,
        ),
    ] {
        assert_eq!(
            lower_as("agony", |row| {
                let operation = &mut row["effect"]["operation"];
                operation["duration_ms"] = Value::Null;
                operation["condition"]["lifetime"] = json!("DamageSchedule");
                operation["condition"]["damage_over_time"] = json!({
                    "tick_profile": "Fixed",
                    "first_tick": "Immediate",
                    "ticks": ticks,
                });
            })
            .unwrap_err(),
            expected
        );
    }
}
