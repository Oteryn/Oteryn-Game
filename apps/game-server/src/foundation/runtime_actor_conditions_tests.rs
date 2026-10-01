#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::super::super::exact_actor_test_ability::condition::{
    ConditionValues, ConflictKey, SpeedRange,
};
use super::*;
use oteryn_simulation_determinism::GameplayDecisionRoot;

fn fixture() -> (
    ChannelRuntimeV1,
    ExactActorRef,
    GameSessionId,
    ExactActorRef,
) {
    let mut runtime = super::super::tests::runtime(2);
    let session = super::super::tests::session(4);
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reservation).unwrap();
    let creature = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        })
        .unwrap();
    (runtime, player, session, creature)
}
fn definition(name: &str) -> ConditionDefinition {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/condition-authoring/authored-conditions.json"
    ))
    .unwrap();
    let row = rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == format!("charm.{name}"))
        .unwrap();
    let values = &row["speed"];
    let n = |key: &str| i32::try_from(values[key].as_i64().unwrap()).unwrap();
    ConditionDefinition::new(
        &format!("oteryn:condition.charm.{name}"),
        1,
        ConditionValues::Speed {
            paralysis: values["kind"] == "paralysis",
            range: SpeedRange {
                a_min: n("a_min"),
                b_min: n("b_min"),
                a_max: n("a_max"),
                b_max: n("b_max"),
            },
            duration_ms: u32::try_from(values["duration_ms"].as_u64().unwrap()).unwrap(),
        },
    )
    .unwrap()
}
fn facts<'a>(
    root: &'a GameplayDecisionRoot,
    now: u64,
    player: bool,
    occurrence: u8,
) -> ApplicationFacts<'a> {
    ApplicationFacts {
        now,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: player,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([occurrence; 16]),
    }
}
fn at(value: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(value)
}

#[test]
fn native_cleanse_removes_once_and_keeps_key_immunity_until_owner_expiry() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let definition = definition("cripple");
    let applied = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Apply {
                definition: &definition,
                source: ConditionSource {
                    actor: creature,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts: facts(&root, 0, true, 1),
            },
            at(0),
        )
        .unwrap();
    runtime.commit_actor_condition(&applied, at(0)).unwrap();
    let cleanse = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Cleanse(facts(&root, 1_000, true, 2)),
            at(1_000),
        )
        .unwrap();
    assert!(runtime.commit_actor_condition(&cleanse, at(1_000)).unwrap());
    assert!(!runtime.commit_actor_condition(&cleanse, at(2_000)).unwrap());
    assert_eq!(
        runtime.prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Cleanse(facts(&root, 2_000, true, 2)),
            at(2_000),
        ),
        Err(ConditionOwnerError::StalePlan)
    );
    runtime
        .record_control_loss(
            player,
            session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 4000,
            },
        )
        .unwrap();
    runtime.restore_control(player, session, 1).unwrap();
    let store = runtime.actor_conditions(player, Some(session)).unwrap();
    assert!(store.instances().is_empty());
    assert_eq!(
        store.cleanse_immunity_remaining(ConflictKey::Speed, 2_000),
        10_999_000
    );
    assert!(
        runtime
            .condition_state(runtime.condition_index(player, Some(session)).unwrap())
            .receipt
            .is_some()
    );
    let deadline = 11_001_000;
    let expiry = runtime
        .prepare_actor_condition_expiry(
            player,
            Some(session),
            DecisionOccurrenceId::from_bytes([3; 16]),
            at(deadline),
        )
        .unwrap();
    assert!(
        runtime
            .commit_actor_condition(&expiry, at(deadline))
            .unwrap()
    );
    assert!(
        !runtime
            .commit_actor_condition(&expiry, at(deadline + 1))
            .unwrap()
    );
    assert_eq!(
        runtime.commit_actor_condition(&cleanse, at(deadline + 1)),
        Err(ConditionOwnerError::StalePlan)
    );
    let haste = self::definition("adrenaline_burst");
    let apply = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Apply {
                definition: &haste,
                source: ConditionSource {
                    actor: player,
                    session: Some(session),
                    kind: ConditionSourceKind::SelfUse,
                },
                immunities: &[],
                facts: facts(&root, deadline + 1, true, 4),
            },
            at(deadline + 1),
        )
        .unwrap();
    assert!(
        runtime
            .commit_actor_condition(&apply, at(deadline + 1))
            .unwrap()
    );
    assert!(
        runtime
            .actor_active_speed_delta(player, Some(session), at(deadline + 1))
            .unwrap()
            > 0
    );
}

#[test]
fn stale_session_generation_scope_and_future_time_never_mutate_native_conditions() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let definition = definition("cripple");
    let source = ConditionSource {
        actor: player,
        session: Some(session),
        kind: ConditionSourceKind::Player,
    };
    let command = |now| ActorConditionTransition::Apply {
        definition: &definition,
        source,
        immunities: &[],
        facts: facts(&root, now, false, 1),
    };
    assert_eq!(
        runtime.prepare_actor_condition(creature, None, command(1), at(0)),
        Err(ConditionOwnerError::TimeMismatch)
    );
    let plan = runtime
        .prepare_actor_condition(creature, None, command(1), at(1))
        .unwrap();
    let before = runtime.carrier.slots.clone();
    assert_eq!(
        runtime.commit_actor_condition(&plan, at(0)),
        Err(ConditionOwnerError::TimeMismatch)
    );
    let wrong =
        GameSessionId::decode(&[0x70, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).unwrap();
    assert!(runtime.actor_conditions(player, Some(wrong)).is_err());
    let mut foreign_session = plan.clone();
    foreign_session.receipt.source.as_mut().unwrap().session = Some(wrong);
    assert!(
        runtime
            .commit_actor_condition(&foreign_session, at(1))
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
    let mut other = fixture().0;
    other.continuity.current_generation =
        super::super::super::ScopeOwnershipGeneration::new(2).unwrap();
    assert_eq!(
        other.commit_actor_condition(&plan, at(1)),
        Err(ConditionOwnerError::Actor(CarrierError::WrongScope))
    );
    runtime.remove_test_actor(creature).unwrap();
    let replacement = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        })
        .unwrap();
    assert!(runtime.commit_actor_condition(&plan, at(1)).is_err());
    assert!(
        runtime
            .actor_conditions(replacement, None)
            .unwrap()
            .instances()
            .is_empty()
    );
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .is_empty()
    );
}

#[test]
fn native_authored_speed_refresh_replacement_and_exact_expiry_retain_original_plans() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let apply_at = |runtime: &mut ChannelRuntimeV1, name: &str, now, occurrence| {
        let definition = definition(name);
        let plan = runtime
            .prepare_actor_condition(
                player,
                Some(session),
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor: creature,
                        session: None,
                        kind: ConditionSourceKind::Creature,
                    },
                    immunities: &[],
                    facts: facts(&root, now, true, occurrence),
                },
                at(now),
            )
            .unwrap();
        assert!(runtime.commit_actor_condition(&plan, at(now)).unwrap());
        plan
    };
    let first = apply_at(&mut runtime, "cripple", 0, 3);
    let superseded_expiry = runtime
        .prepare_actor_condition_expiry(
            player,
            Some(session),
            DecisionOccurrenceId::from_bytes([7; 16]),
            at(10_000_000),
        )
        .unwrap();
    let delta = runtime
        .actor_active_speed_delta(player, Some(session), at(0))
        .unwrap();
    assert!(delta < 0);
    let refreshed = apply_at(&mut runtime, "numb", 5_000_000, 4);
    assert_eq!(
        runtime.commit_actor_condition(&superseded_expiry, at(10_000_000)),
        Err(ConditionOwnerError::StalePlan)
    );
    assert_eq!(
        runtime
            .actor_active_speed_delta(player, Some(session), at(10_000_000))
            .unwrap(),
        delta
    );
    assert_eq!(
        runtime.commit_actor_condition(&first, at(5_000_000)),
        Err(ConditionOwnerError::StalePlan)
    );
    let replacement = apply_at(&mut runtime, "adrenaline_burst", 6_000_000, 5);
    assert_eq!(
        runtime.commit_actor_condition(&refreshed, at(6_000_000)),
        Err(ConditionOwnerError::StalePlan)
    );
    assert!(
        runtime
            .actor_active_speed_delta(player, Some(session), at(15_999_999))
            .unwrap()
            > 0
    );
    assert_eq!(
        runtime
            .actor_active_speed_delta(player, Some(session), at(16_000_000))
            .unwrap(),
        0
    );
    let expiry = runtime
        .prepare_actor_condition_expiry(
            player,
            Some(session),
            DecisionOccurrenceId::from_bytes([6; 16]),
            at(16_000_000),
        )
        .unwrap();
    assert_eq!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .len(),
        1
    );
    assert!(
        runtime
            .commit_actor_condition(&expiry, at(16_000_000))
            .unwrap()
    );
    assert!(
        !runtime
            .commit_actor_condition(&expiry, at(17_000_000))
            .unwrap()
    );
    assert_eq!(
        runtime.commit_actor_condition(&replacement, at(17_000_000)),
        Err(ConditionOwnerError::StalePlan)
    );
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .is_empty()
    );
}

#[test]
fn native_expiry_revalidates_session_generation_and_monotonic_time_at_commit() {
    let (mut runtime, player, session, creature) = fixture();
    let occurrence = DecisionOccurrenceId::from_bytes([9; 16]);
    let plan = runtime
        .prepare_actor_condition_expiry(player, Some(session), occurrence, at(1))
        .unwrap();
    let before = runtime.carrier.slots.clone();
    assert_eq!(
        runtime.commit_actor_condition(&plan, at(0)),
        Err(ConditionOwnerError::TimeMismatch)
    );
    let wrong =
        GameSessionId::decode(&[0x70, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).unwrap();
    let mut foreign_session = plan.clone();
    foreign_session.session = Some(wrong);
    assert!(
        runtime
            .commit_actor_condition(&foreign_session, at(1))
            .is_err()
    );
    assert!(
        runtime
            .actor_active_speed_delta(player, Some(wrong), at(1))
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
    assert!(runtime.commit_actor_condition(&plan, at(1)).unwrap());
    assert_eq!(
        runtime.actor_active_speed_delta(player, Some(session), at(0)),
        Err(ConditionOwnerError::TimeMismatch)
    );
    assert_eq!(
        runtime.prepare_actor_condition_expiry(
            player,
            Some(session),
            DecisionOccurrenceId::from_bytes([10; 16]),
            at(0)
        ),
        Err(ConditionOwnerError::TimeMismatch)
    );
    assert_eq!(
        runtime.prepare_actor_condition_expiry(player, Some(session), occurrence, at(1)),
        Err(ConditionOwnerError::StalePlan)
    );
    let stale = runtime
        .prepare_actor_condition_expiry(creature, None, occurrence, at(1))
        .unwrap();
    runtime.remove_test_actor(creature).unwrap();
    let replacement = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        })
        .unwrap();
    assert!(runtime.commit_actor_condition(&stale, at(1)).is_err());
    assert!(
        runtime
            .actor_active_speed_delta(creature, None, at(1))
            .is_err()
    );
    assert_eq!(
        runtime.actor_active_speed_delta(replacement, None, at(1)),
        Ok(0)
    );
}
