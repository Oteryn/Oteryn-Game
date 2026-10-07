#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::foundation::{
    ApplicationFacts, ConditionRefusal, ConditionSourceKind, ConditionStore, ConditionType,
    TickFacts, TickKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use serde_json::{Value, json};
fn rows() -> Vec<Value> {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/agents/evidence/monster-runtime-project-20261004/lanes/special/condition-fixtures.json"))).unwrap()
}
fn effect(row: &Value) -> ProjectV2InlineEffect {
    serde_json::from_value(row["effect"].clone()).unwrap()
}
fn facts<'a>(root: &'a GameplayDecisionRoot) -> ApplicationFacts<'a> {
    ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    }
}
fn lower(row: &Value) -> Result<ConditionDefinition, ConditionContentError> {
    let formula: Option<ProjectV2FormulaAuthoring> = row
        .get("formula")
        .map(|v| serde_json::from_value(v.clone()).unwrap());
    let reference: Option<ProjectV2DefinitionRef> = row
        .get("formula_ref")
        .map(|v| serde_json::from_value(v.clone()).unwrap());
    lower_condition_definition(&effect(row), 1, reference.as_ref().zip(formula.as_ref()))
}
#[test]
fn source_bound_condition_population_executes_existing_store_only() {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let mut accepted = 0;
    let mut rejected = 0;
    let mut results = Vec::new();
    for row in rows() {
        match lower(&row) {
            Ok(definition) => {
                let mut store = ConditionStore::<u32>::new();
                let f = facts(&root);
                assert!(
                    store
                        .apply(&definition, Some(9), ConditionSourceKind::Creature, &[], &f)
                        .is_ok()
                );
                assert_eq!(store.instances().len(), 1);
                match definition.values() {
                    ConditionValues::Speed {
                        duration_ms,
                        paralysis,
                        ..
                    }
                    | ConditionValues::RationalSpeed {
                        duration_ms,
                        paralysis,
                        ..
                    } => {
                        if paralysis {
                            assert!(220 + store.speed_delta() >= 40);
                        }
                        assert!(
                            store
                                .take_due(u64::from(duration_ms) * 1000 - 1, TickFacts::default())
                                .is_empty()
                        );
                        assert_eq!(store.instances().len(), 1);
                        store.take_due(u64::from(duration_ms) * 1000, TickFacts::default());
                        assert!(store.instances().is_empty());
                        assert_eq!(store.speed_delta(), 0);
                    }
                    ConditionValues::DamageSchedule { .. } => {
                        let mut now = 0_u64;
                        let mut total = 0_u64;
                        while !store.instances().is_empty() {
                            let ticks = store.take_due(now, TickFacts::default());
                            for tick in ticks {
                                match tick.kind {
                                    TickKind::Damage {
                                        amount,
                                        refused: false,
                                        ..
                                    } => total += u64::from(amount),
                                    other => assert!(
                                        matches!(other, TickKind::Damage { refused: false, .. }),
                                        "invalid scheduled damage result"
                                    ),
                                }
                            }
                            assert!(
                                store.take_due(now, TickFacts::default()).is_empty(),
                                "schedule tick replay duplicated"
                            );
                            now += 1_000_000;
                            assert!(now < 1_000_000_000, "source schedule failed to finish");
                        }
                        assert!(total > 0);
                    }
                    ConditionValues::DamageOverTime {
                        total_min,
                        total_max,
                        per_tick,
                        interval_ms,
                        delayed,
                        ..
                    } => {
                        assert_eq!(total_min, total_max);
                        let count = total_min / per_tick;
                        let mut sum = 0;
                        let origin = if delayed {
                            u64::from(interval_ms) * 1000
                        } else {
                            0
                        };
                        if origin > 0 {
                            assert!(store.take_due(origin - 1, TickFacts::default()).is_empty());
                        }
                        for i in 0..count {
                            let now = origin + u64::from(i) * u64::from(interval_ms) * 1000;
                            let ticks = store.take_due(now, TickFacts::default());
                            assert_eq!(ticks.len(), 1);
                            match ticks[0].kind {
                                TickKind::Damage {
                                    amount, refused, ..
                                } => {
                                    assert_eq!(amount, per_tick);
                                    assert!(!refused);
                                    sum += amount;
                                }
                                other => assert!(
                                    matches!(other, TickKind::Damage { .. }),
                                    "wrong tick family"
                                ),
                            }
                            assert!(
                                store.take_due(now, TickFacts::default()).is_empty(),
                                "tick replay duplicated"
                            );
                        }
                        assert_eq!(sum, total_min);
                        assert!(store.instances().is_empty());
                    }
                    ConditionValues::SourceAttributes {
                        modifiers,
                        duration_ms,
                    } => {
                        assert_eq!(store.effective_attributes(0), modifiers);
                        for skill in [
                            crate::foundation::CombatSkill::Club,
                            crate::foundation::CombatSkill::Distance,
                        ] {
                            assert!(modifiers.combat_skill(skill, 100).is_some());
                        }
                        assert!(modifiers.magic_level(100).is_some());
                        assert_eq!(
                            store.effective_attributes(u64::from(duration_ms) * 1000),
                            AttributeModifiers::default()
                        );
                    }
                    ConditionValues::TimedStatus { kind, duration_ms } => {
                        assert!(store.has_status(kind, 0));
                        assert!(store.has_status(kind, u64::from(duration_ms) * 1000 - 1));
                        assert!(!store.has_status(kind, u64::from(duration_ms) * 1000));
                        assert!(
                            store
                                .take_due(u64::from(duration_ms) * 1000, TickFacts::default())
                                .is_empty()
                        );
                        assert!(store.instances().is_empty());
                    }
                    other => assert!(
                        matches!(
                            other,
                            ConditionValues::Speed { .. }
                                | ConditionValues::RationalSpeed { .. }
                                | ConditionValues::DamageSchedule { .. }
                                | ConditionValues::DamageOverTime { .. }
                                | ConditionValues::SourceAttributes { .. }
                                | ConditionValues::TimedStatus { .. }
                        ),
                        "unqualified definition family"
                    ),
                }
                accepted += 1;
                results.push(json!({"ability":row["ability"],"effect":row["effect"]["key"],"status":"CONDITION_STORE_EXECUTED","owner_health_mutated":false,"movement_executed":false}));
            }
            Err(error) => {
                rejected += 1;
                results.push(json!({"ability":row["ability"],"effect":row["effect"]["key"],"status":"FAIL_CLOSED","error":format!("{error:?}")}));
            }
        }
    }
    assert!(accepted > 0);
    assert_eq!(accepted, rows().len());
    assert_eq!(rejected, 0);
    if let Ok(path) = std::env::var("OTERYN_CONDITION_QUALIFICATION_OUTPUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"rows":results,"accepted":accepted,"rejected":rejected,"condition_store_execution":true,"runtime_owner_wiring":false,"actor_health_mutations":false,"movement_execution":false,"presentation_execution":false,"special_casts_executed":false})).unwrap()).unwrap();
    }
}
#[test]
fn unknown_root_fear_fail_closed_and_source_schedules_are_supported() {
    let rows = rows();
    for kind in ["rooted", "feared", "unknown_condition"] {
        let mut row = rows
            .iter()
            .find(|r| r["effect"]["operation"]["condition"]["condition_type"] == "haste")
            .unwrap()
            .clone();
        row["effect"]["operation"]["condition"]["condition_type"] = json!(kind);
        assert!(lower(&row).is_err());
    }
    for row in &rows {
        let condition = &row["effect"]["operation"]["condition"];
        if matches!(
            condition["damage_over_time"]["tick_profile"].as_str(),
            Some("Decreasing" | "Geometric")
        ) {
            assert!(
                lower(row).is_ok(),
                "source-generated schedule must lower exactly"
            );
        }
    }
}
#[test]
fn exact_formula_identity_and_coefficient_required() {
    let row = rows()
        .into_iter()
        .find(|r| r["effect"]["operation"]["condition"]["condition_type"] == "haste")
        .unwrap();
    let e = effect(&row);
    assert_eq!(
        lower_condition_definition(&e, 1, None),
        Err(ConditionContentError::MissingOrMismatchedFormula)
    );
    let mut wrong: ProjectV2DefinitionRef =
        serde_json::from_value(row["formula_ref"].clone()).unwrap();
    wrong.key.push_str("_wrong");
    let f: ProjectV2FormulaAuthoring = serde_json::from_value(row["formula"].clone()).unwrap();
    assert_eq!(
        lower_condition_definition(&e, 1, Some((&wrong, &f))),
        Err(ConditionContentError::MissingOrMismatchedFormula)
    );
    assert_eq!(
        exact_thousandths(ProjectV2ExactRatio {
            numerator: 1,
            denominator: 3
        }),
        Err(ConditionContentError::InexactSpeedCoefficient)
    );
    assert_eq!(
        exact_thousandths(ProjectV2ExactRatio {
            numerator: 1,
            denominator: 0
        }),
        Err(ConditionContentError::InvalidDefinition)
    );
}
#[test]
fn immunity_and_reentry_refuse_actual_store_mutation() {
    let row = rows()
        .into_iter()
        .find(|r| {
            r["effect"]["operation"]["condition"]["condition_type"] == "paralyze"
                && lower(r).is_ok()
        })
        .unwrap();
    let definition = lower(&row).unwrap();
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let mut f = facts(&root);
    let mut store = ConditionStore::<u32>::new();
    assert_eq!(
        store.apply(
            &definition,
            Some(9),
            ConditionSourceKind::Creature,
            &[ConditionType::Paralysis],
            &f
        ),
        Err(ConditionRefusal::Immune)
    );
    assert!(store.instances().is_empty());
    f.target_reentry_protected = true;
    assert_eq!(
        store.apply(&definition, Some(9), ConditionSourceKind::Creature, &[], &f),
        Err(ConditionRefusal::ReentryProtected)
    );
    assert!(store.instances().is_empty());
}
#[test]
fn zero_short_interval_and_extra_payload_are_rejected() {
    let row = rows()
        .into_iter()
        .find(|r| {
            r["effect"]["operation"]["condition"]["damage_over_time"]["tick_profile"] == "Fixed"
                && lower(r).is_ok()
        })
        .unwrap();
    for (field, value) in [("amount", 0), ("interval_ms", 999), ("count", 0)] {
        let mut bad = row.clone();
        bad["effect"]["operation"]["condition"]["damage_over_time"]["ticks"][0][field] =
            json!(value);
        assert!(lower(&bad).is_err());
    }
    let mut bad = row.clone();
    bad["effect"]["operation"]["condition"]["buff_spell"] = json!(true);
    assert_eq!(
        lower(&bad),
        Err(ConditionContentError::UnsupportedParameters)
    );
    let mut bad = row;
    bad["effect"]["operation"]["duration_ms"] = json!(3000);
    assert_eq!(
        lower(&bad),
        Err(ConditionContentError::UnsupportedParameters)
    );
}
#[test]
fn revision_duration_and_source_key_fail_closed() {
    let row = rows()
        .into_iter()
        .find(|r| {
            r["effect"]["operation"]["condition"]["condition_type"] == "haste" && lower(r).is_ok()
        })
        .unwrap();
    let formula: ProjectV2FormulaAuthoring =
        serde_json::from_value(row["formula"].clone()).unwrap();
    let reference: ProjectV2DefinitionRef =
        serde_json::from_value(row["formula_ref"].clone()).unwrap();
    let e = effect(&row);
    assert_eq!(
        lower_condition_definition(&e, 0, Some((&reference, &formula))),
        Err(ConditionContentError::InvalidDefinition)
    );
    for duration in [0, u64::from(u32::MAX) + 1] {
        let mut bad = row.clone();
        bad["effect"]["operation"]["duration_ms"] = json!(duration);
        assert!(lower(&bad).is_err());
    }
    let mut bad = row.clone();
    bad["effect"]["operation"]
        .as_object_mut()
        .unwrap()
        .remove("duration_ms");
    assert!(lower(&bad).is_err());
    let mut bad = row;
    bad["effect"]["key"] = json!("bad key with spaces");
    assert_eq!(lower(&bad), Err(ConditionContentError::InvalidDefinition));
}

#[test]
fn all_ten_source_rational_speed_definitions_preserve_coefficients() {
    let mut count = 0;
    for row in rows() {
        let Some(formula) = row.get("formula") else {
            continue;
        };
        let f: ProjectV2FormulaAuthoring = serde_json::from_value(formula.clone()).unwrap();
        let ProjectV2FormulaAuthoring::SpeedModifier {
            minimum_multiplier,
            maximum_multiplier,
            ..
        } = f
        else {
            continue;
        };
        if exact_thousandths(minimum_multiplier)
            != Err(ConditionContentError::InexactSpeedCoefficient)
            && exact_thousandths(maximum_multiplier)
                != Err(ConditionContentError::InexactSpeedCoefficient)
        {
            continue;
        }
        let definition = lower(&row).unwrap();
        let range = match definition.values() {
            ConditionValues::RationalSpeed { range, .. } => Some(range),
            _ => None,
        }
        .expect("lossy source speed payload must retain RationalSpeed coefficients");
        assert_eq!(range.a_min.numerator, minimum_multiplier.numerator);
        assert_eq!(range.a_min.denominator, minimum_multiplier.denominator);
        assert_eq!(range.a_max.numerator, maximum_multiplier.numerator);
        assert_eq!(range.a_max.denominator, maximum_multiplier.denominator);
        count += 1;
    }
    assert_eq!(count, 10);
}

// Source C++ ConditionSpeed formula endpoints are TARGET, not delta.
#[test]
fn pinned_source_speed_target_endpoints_subtract_current_base_once() {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    for (numerator, denominator, base, expected_target, expected_delta) in [
        (3, 10, 220, 94, -126),  // named speedChange -400 lower endpoint
        (3, 5, 220, 148, -72),   // named speedChange -400 upper endpoint
        (3, 10, 320, 124, -196), // independently supplied new base
        (0, 1, 220, 40, -180),   // source clamped speedChange -1000
        (1, 1, 220, 220, 0),     // source haste speedChange1000 lower endpoint
        (2, 1, 220, 400, 180),   // source haste speedChange1000 upper endpoint
    ] {
        let mut row = rows()
            .into_iter()
            .find(|r| r.get("formula").is_some())
            .unwrap();
        let mut formula = row["formula"].clone();
        formula["minimum_multiplier"] = json!({"numerator":numerator,"denominator":denominator});
        formula["maximum_multiplier"] = formula["minimum_multiplier"].clone();
        formula["minimum_offset"] = json!(40);
        formula["maximum_offset"] = json!(40);
        row["formula"] = formula;
        let definition = lower(&row).unwrap();
        let mut f = facts(&root);
        f.base_speed = base;
        let mut store = ConditionStore::<u32>::new();
        store
            .apply(&definition, Some(9), ConditionSourceKind::Creature, &[], &f)
            .unwrap();
        assert_eq!(store.speed_delta(), expected_delta);
        assert_eq!(i64::from(base) + store.speed_delta(), expected_target);
    }
}

#[test]
fn source_speed_requires_resolved_exact_formula_reference_before_store() {
    let row = rows()
        .into_iter()
        .find(|r| r.get("formula").is_some())
        .unwrap();
    let e = effect(&row);
    assert_eq!(
        lower_condition_definition(&e, 1, None),
        Err(ConditionContentError::MissingOrMismatchedFormula)
    );
    let formula: ProjectV2FormulaAuthoring =
        serde_json::from_value(row["formula"].clone()).unwrap();
    let mut reference: ProjectV2DefinitionRef =
        serde_json::from_value(row["formula_ref"].clone()).unwrap();
    reference.revision = "substituted-r2".into();
    assert_eq!(
        lower_condition_definition(&e, 1, Some((&reference, &formula))),
        Err(ConditionContentError::MissingOrMismatchedFormula)
    );
}
