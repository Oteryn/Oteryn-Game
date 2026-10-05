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

#[test]
fn canonical_batch_has_explicit_immune_outcomes_and_retained_replay() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([17; 32]);
    let defs = [definition("cripple"), definition("adrenaline_burst")];
    let source = ConditionSource {
        actor: creature,
        session: None,
        kind: ConditionSourceKind::Creature,
    };
    let plan = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::ApplyBatch {
                definitions: &defs,
                source,
                immunities: &[ConditionType::Paralysis],
                facts: facts(&root, 0, true, 71),
            },
            at(0),
        )
        .unwrap();
    assert_eq!(plan.applications().len(), 2);
    assert_eq!(plan.applications()[0], Err(ConditionRefusal::Immune));
    assert!(plan.applications()[1].is_ok());
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .is_empty()
    );
    assert!(runtime.commit_actor_condition(&plan, at(0)).unwrap());
    assert!(!runtime.commit_actor_condition(&plan, at(1)).unwrap());
    assert_eq!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .len(),
        1
    );
    assert_eq!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()[0]
            .definition(),
        &defs[1]
    );
}

#[test]
fn canonical_batch_refuses_size_before_any_publication_and_revalidates_source() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([18; 32]);
    let defs = vec![definition("cripple"); 17];
    let source = ConditionSource {
        actor: creature,
        session: None,
        kind: ConditionSourceKind::Creature,
    };
    let invalid = runtime.prepare_actor_condition(
        player,
        Some(session),
        ActorConditionTransition::ApplyBatch {
            definitions: &defs,
            source,
            immunities: &[],
            facts: facts(&root, 0, true, 72),
        },
        at(0),
    );
    assert_eq!(invalid, Err(ConditionOwnerError::BatchSize));
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .is_empty()
    );
    let defs = [definition("cripple")];
    let plan = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::ApplyBatch {
                definitions: &defs,
                source,
                immunities: &[],
                facts: facts(&root, 0, true, 73),
            },
            at(0),
        )
        .unwrap();
    runtime.remove_test_actor(creature).unwrap();
    assert!(runtime.commit_actor_condition(&plan, at(0)).is_err());
    assert!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()
            .is_empty()
    );
}

#[test]
fn canonical_type_cure_preserves_shared_speed_sibling_and_stales_old_plan() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([19; 32]);
    let haste = definition("adrenaline_burst");
    let source = ConditionSource {
        actor: creature,
        session: None,
        kind: ConditionSourceKind::Creature,
    };
    let apply = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Apply {
                definition: &haste,
                source,
                immunities: &[],
                facts: facts(&root, 0, true, 74),
            },
            at(0),
        )
        .unwrap();
    runtime.commit_actor_condition(&apply, at(0)).unwrap();
    let cure = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::RemoveType {
                kind: ConditionType::Paralysis,
                source,
                facts: facts(&root, 1, true, 75),
            },
            at(1),
        )
        .unwrap();
    assert!(runtime.commit_actor_condition(&cure, at(1)).unwrap());
    assert!(!runtime.commit_actor_condition(&cure, at(2)).unwrap());
    assert_eq!(
        runtime
            .actor_conditions(player, Some(session))
            .unwrap()
            .instances()[0]
            .definition(),
        &haste
    );
    assert_eq!(
        runtime.commit_actor_condition(&apply, at(2)),
        Err(ConditionOwnerError::StalePlan)
    );
}

#[test]
fn canonical_tick_projection_is_abandonable_without_consuming_live_schedule() {
    let (mut runtime, player, session, creature) = fixture();
    let root = GameplayDecisionRoot::from_bytes([19; 32]);
    let definition = ConditionDefinition::new_damage_schedule(
        "source.dot",
        1,
        DotElement::Poison,
        DamageSchedule::Fixed {
            segments: vec![DamageSegment {
                count: 8,
                interval_ms: 1000,
                amount: 3,
            }],
            delayed: true,
        },
    )
    .unwrap();
    let apply = runtime
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
                facts: facts(&root, 0, true, 81),
            },
            at(0),
        )
        .unwrap();
    runtime.commit_actor_condition(&apply, at(0)).unwrap();
    let before = runtime
        .actor_conditions(player, Some(session))
        .unwrap()
        .clone();
    let occurrence = DecisionOccurrenceId::from_bytes([82; 16]);
    let abandoned = runtime
        .prepare_actor_condition_ticks(
            player,
            Some(session),
            occurrence,
            TickFacts::default(),
            at(8000000),
        )
        .unwrap();
    assert_eq!(abandoned.ticks().len(), 4);
    assert_eq!(
        runtime.actor_conditions(player, Some(session)).unwrap(),
        &before
    );
    let retry = runtime
        .prepare_actor_condition_ticks(
            player,
            Some(session),
            occurrence,
            TickFacts::default(),
            at(8000000),
        )
        .unwrap();
    assert_eq!(retry, abandoned);
    runtime.remove_test_actor(creature).unwrap();
    // The caster is frozen provenance, not new tick-time authority. No HP mutation is performed.
    assert_eq!(
        runtime
            .prepare_actor_condition_ticks(
                player,
                Some(session),
                occurrence,
                TickFacts::default(),
                at(8000000)
            )
            .unwrap(),
        retry
    );
    let expiry = runtime
        .prepare_actor_condition_expiry(
            player,
            Some(session),
            DecisionOccurrenceId::from_bytes([83; 16]),
            at(8000000),
        )
        .unwrap();
    runtime
        .commit_actor_condition(&expiry, at(8000000))
        .unwrap();
    assert_eq!(
        runtime.commit_actor_condition(retry.condition_plan(), at(8000000)),
        Err(ConditionOwnerError::StalePlan)
    );
    assert_eq!(
        runtime.actor_conditions(player, Some(session)).unwrap(),
        &before
    );
}

#[test]
fn source_creature_heal_cure_preflights_all_targets_and_preserves_other_conditions() {
    let mut r = super::super::tests::runtime(2);
    let actor = ExactActorRef(
        r.carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 5)
            .unwrap(),
    );
    let root = GameplayDecisionRoot::from_bytes([81; 32]);
    let definitions = [
        ConditionDefinition::new(
            "source.paralysis",
            1,
            ConditionValues::Speed {
                paralysis: true,
                range: SpeedRange {
                    a_min: 0,
                    a_max: 0,
                    b_min: 40,
                    b_max: 40,
                },
                duration_ms: 1000,
            },
        )
        .unwrap(),
        ConditionDefinition::new(
            "source.invisible",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                duration_ms: 1000,
            },
        )
        .unwrap(),
    ];
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 110,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: false,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([82; 16]),
    };
    let plan = r
        .prepare_actor_condition(
            actor,
            None,
            ActorConditionTransition::ApplyBatch {
                definitions: &definitions,
                source: ConditionSource {
                    actor,
                    session: None,
                    kind: ConditionSourceKind::SelfUse,
                },
                immunities: &[],
                facts,
            },
            at(0),
        )
        .unwrap();
    r.commit_actor_condition(&plan, at(0)).unwrap();
    let before = r.actor_conditions(actor, None).unwrap().clone();
    let content = r.content_pin().server_artifact_digest();
    assert!(
        r.commit_source_creature_heal_and_cure(
            content,
            &[(actor, "wrong".into(), 20, 1)],
            &[actor],
            1
        )
        .is_err()
    );
    assert_eq!(r.actor_conditions(actor, None).unwrap(), &before);
    let index = r.carrier.validate_ref(&r.continuity, actor.0).unwrap();
    assert!(matches!(
        r.carrier.slots[index],
        Slot::CreatureOccupied { health: 5, .. }
    ));
    assert!(
        r.commit_source_creature_heal_and_cure(content, &[], &[actor, actor], 1)
            .is_err()
    );
    assert_eq!(r.actor_conditions(actor, None).unwrap(), &before);
    let receipt = r
        .commit_source_creature_heal_and_cure(
            content,
            &[(actor, "oteryn:creature.rat".into(), 20, 1)],
            &[actor],
            1,
        )
        .unwrap();
    assert_eq!(receipt[0].health_after, 6);
    let store = r.actor_conditions(actor, None).unwrap();
    assert!(store.get(ConflictKey::Speed).is_none());
    assert!(store.has_status(StatusKind::Invisible, 1));
    assert_eq!(
        r.commit_actor_condition(&plan, at(1)),
        Err(ConditionOwnerError::StalePlan)
    );
}
