//! CONDITIONS-0 §3.5 (MAGIC-COND-1): content lowers the gate families by name; `agony` and
//! any other unknown condition type stay unsupported.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use super::*;
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
fn agony_unknown_names_and_extra_gate_payload_fail_closed() {
    for kind in ["agony", "mute", "Pacified", ""] {
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
