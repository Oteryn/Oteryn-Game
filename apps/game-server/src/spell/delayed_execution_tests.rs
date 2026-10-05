use super::*;
use crate::foundation::{CommandId, MovementLocalPosition};
use crate::spell::combat_batch::{MonsterAiState, OwnerCombatChange, OwnerCombatEffect};
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#[test]
fn actual_owner_due_applies_ai_lifetime_at_callback_and_never_replays() {
    // Two independent real player owners may share the Channel timer lane.
    // A retained unknown command does not authorize a newer command on that
    // same player, so lane refresh is exercised by another admitted player.
    let id = |n| [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n];
    let world = crate::foundation::WorldId::decode(&id(60)).unwrap();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        crate::foundation::ChannelId::decode(&id(61)).unwrap(),
        crate::foundation::NodeId::decode(&id(62)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        crate::foundation::ChannelContentPin::test(world),
    )
    .unwrap();
    let session = GameSessionId::decode(&id(91)).unwrap();
    let admission = runtime.reserve_fresh_session(session).unwrap();
    let caster = runtime.commit_fresh_session(admission).unwrap();
    let other_session = GameSessionId::decode(&id(93)).unwrap();
    let admission = runtime.reserve_fresh_session(other_session).unwrap();
    let other_caster = runtime.commit_fresh_session(admission).unwrap();
    let target = runtime
        .admit_test_creature(MovementLocalPosition {
            x: 1,
            y: 1,
            floor: 7,
        })
        .unwrap();
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .unwrap();
    let row = document["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Chivalrous Challenge")
        .unwrap();
    let spell = super::super::native::spell_from_bundle(
        &serde_json::json!({"spell":row["spell"]}),
        &row["dependencies"],
    )
    .unwrap();
    let mut character = *session.as_bytes();
    character[15] = 92;
    let attacker = CharacterId::decode(&character).unwrap();
    let occurrence = AbilityOccurrence::new(
        "spell-timer:actual-owner",
        crate::ability::RevisionSet::new(
            "rules:1",
            "content:1",
            "world:1",
            "formula:1",
            "simulation:1",
        )
        .unwrap(),
    )
    .unwrap();
    let position = TilePosition {
        x: 0,
        y: 0,
        floor: 7,
    };
    let binding = CastBinding {
        spell,
        caster,
        attacker,
        command: CommandRef::new(session, CommandId::new(1).unwrap()),
        occurrence,
        parent_binding: b"qualified-cast:chivalrous-challenge".to_vec(),
        cast_at: SemanticTimeMicros::from_micros(0),
        cast_position: position,
        cast_snapshot: None,
    };
    let mut owner = SpellTimerOwner::new(
        RuntimeScopeRefV1::channel(runtime.binding().world_id(), runtime.binding().channel_id()),
        runtime.binding().scope_generation(),
    )
    .unwrap();
    let original = OwnerCombatBatch {
        caster,
        attacker,
        current_lease_generation: 1,
        command: binding.command,
        occurrence: binding.occurrence.clone().into(),
        binding: binding.parent_binding.clone(),
        anchor: None,
        now_ms: 0,
        effects: vec![OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::MonsterAi(MonsterAiState {
                forced_distance: None,
                challenged_to: Some((caster, 6000)),
            }),
        }],
        deferred: None,
    };
    let stamp = runtime.issue_owner_work().unwrap();
    let saved = SavedNativeOwnerEffect {
        binding: binding.clone(),
        effect: OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::MonsterAi(MonsterAiState {
                forced_distance: Some((1, 6000)),
                challenged_to: None,
            }),
        },
    };
    let reservation = owner
        .schedule_reservation(
            runtime.owner_fence().unwrap(),
            stamp,
            vec![ScheduleRequest {
                occurrence: SpellTimerOccurrence {
                    command: binding.command,
                    phase: 0,
                },
                due: SemanticTimeMicros::from_micros(100_000),
                payload: TimerPayload::NativeOwnerEffect(saved),
            }],
        )
        .unwrap();
    let mut proof = owner
        .preflight_install(
            runtime.owner_fence().unwrap(),
            stamp,
            &original,
            reservation,
        )
        .unwrap();
    // While an original SQL outcome is retained, another genuine cast can
    // install its own timer. Rebase only the original additions before retry,
    // without overwriting the now-current lane or redrawing either source plan.
    let mut other_binding = binding.clone();
    other_binding.caster = other_caster;
    other_binding.attacker = CharacterId::decode(&id(94)).unwrap();
    other_binding.command = CommandRef::new(other_session, CommandId::new(1).unwrap());
    other_binding.parent_binding = b"unrelated-qualified-ai-cast".to_vec();
    let other_batch = OwnerCombatBatch {
        caster: other_binding.caster,
        attacker: other_binding.attacker,
        command: other_binding.command,
        binding: other_binding.parent_binding.clone(),
        ..original.clone()
    };
    let other_stamp = runtime.issue_owner_work().unwrap();
    let other_reservation = owner
        .schedule_reservation(
            runtime.owner_fence().unwrap(),
            other_stamp,
            vec![ScheduleRequest {
                occurrence: SpellTimerOccurrence {
                    command: other_binding.command,
                    phase: 0,
                },
                due: SemanticTimeMicros::from_micros(200_000),
                payload: TimerPayload::NativeOwnerEffect(SavedNativeOwnerEffect {
                    binding: other_binding,
                    effect: OwnerCombatEffect {
                        target,
                        sub_ordinal: 0,
                        change: OwnerCombatChange::MonsterAi(MonsterAiState {
                            forced_distance: Some((1, 6000)),
                            challenged_to: None,
                        }),
                    },
                }),
            }],
        )
        .unwrap();
    let other_proof = owner
        .preflight_install(
            runtime.owner_fence().unwrap(),
            other_stamp,
            &other_batch,
            other_reservation,
        )
        .unwrap();
    let staged = runtime.stage_spell_batch(&other_batch).unwrap();
    assert!(runtime.commit_spell_batch(staged).unwrap().applied);
    owner.install_preflighted(other_proof);
    assert!(matches!(
        proof.validate_current(&owner, runtime.owner_fence().unwrap(), &original),
        Err(Error::StaleReservation)
    ));
    proof
        .refresh_current(&owner, runtime.owner_fence().unwrap(), &original)
        .unwrap();
    proof
        .validate_current(&owner, runtime.owner_fence().unwrap(), &original)
        .unwrap();
    assert_eq!(owner.pending_len(), 1);
    let staged = runtime.stage_spell_batch(&original).unwrap();
    assert!(runtime.commit_spell_batch(staged).unwrap().applied);
    owner.install_preflighted(proof);
    struct Clock(u64);
    impl OwnerClock for Clock {
        fn now(&self) -> SemanticTimeMicros {
            SemanticTimeMicros::from_micros(self.0)
        }
    }
    let mut build = |_: &ChannelRuntimeV1,
                     payload: &TimerPayload,
                     _: SpellTimerOccurrence,
                     now: SemanticTimeMicros,
                     _: RuntimeWorkStamp| {
        let TimerPayload::NativeOwnerEffect(saved) = payload else {
            return Err(Error::InvalidPayload);
        };
        Ok(OwnerCombatBatch {
            effects: vec![saved.at_due(now)?],
            now_ms: now.get() / 1000,
            ..original.clone()
        })
    };
    let stamp = runtime.issue_owner_work().unwrap();
    assert!(
        owner
            .fire_due_current(&mut runtime, &Clock(99_000), stamp, &mut build)
            .unwrap()
            .receipts
            .is_empty()
    );
    assert_eq!(
        runtime
            .creature_combat_facts(target, 99)
            .unwrap()
            .monster_ai
            .unwrap()
            .forced_distance,
        None
    );
    let stamp = runtime.issue_owner_work().unwrap();
    let fired = owner
        .fire_due_current(&mut runtime, &Clock(100_000), stamp, &mut build)
        .unwrap();
    assert_eq!(fired.receipts.len(), 1);
    assert!(fired.blocked.is_none());
    assert_eq!(
        runtime
            .creature_combat_facts(target, 100)
            .unwrap()
            .monster_ai
            .unwrap()
            .forced_distance,
        Some((1, 6100))
    );
    assert_eq!(owner.pending_len(), 1);
    assert_eq!(owner.payloads[0].occurrence.command, other_batch.command);
    // This actual physical receipt consumed the phase before composition can
    // release its read transaction. Changing the target after that receipt must
    // never retain the old predecessor or execute the consumed callback again.
    assert!(owner.prepared_due.is_none());
    assert!(
        !owner
            .payloads
            .iter()
            .any(|p| p.occurrence.command == original.command)
    );
    let mut later_hit = original.clone();
    later_hit.command = CommandRef::new(session, CommandId::new(2).unwrap());
    later_hit.binding = b"actual-target-change-after-consumed-due-phase".to_vec();
    later_hit.now_ms = 100;
    later_hit.effects = vec![OwnerCombatEffect {
        target,
        sub_ordinal: 0,
        change: OwnerCombatChange::Damage {
            target_atom: runtime.creature_spell_target_atom(target).unwrap(),
            magnitude: 3,
        },
    }];
    let actual = runtime.stage_spell_batch(&later_hit).unwrap();
    assert!(runtime.commit_spell_batch(actual).unwrap().applied);
    assert_eq!(
        runtime.creature_combat_facts(target, 100).unwrap().health,
        17
    );
    assert_eq!(
        owner.payloads[0].due,
        SemanticTimeMicros::from_micros(200_000)
    );
    let stamp = runtime.issue_owner_work().unwrap();
    assert!(
        owner
            .fire_due_current(&mut runtime, &Clock(100_000), stamp, &mut build)
            .unwrap()
            .receipts
            .is_empty()
    );
    assert_eq!(
        runtime.creature_combat_facts(target, 100).unwrap().health,
        17
    );
    let expired = runtime
        .creature_combat_facts(target, 6100)
        .unwrap()
        .monster_ai
        .unwrap();
    assert_eq!(expired.forced_distance, None);
    assert_eq!(expired.challenged_to, None);
    runtime.remove_test_actor(target).unwrap();
    let stamp = runtime.issue_owner_work().unwrap();
    let stale = owner
        .fire_due_current(&mut runtime, &Clock(200_000), stamp, |_, _, _, _, _| {
            panic!("stale physical recipient must be dropped before callback")
        })
        .unwrap();
    assert_eq!(stale.dropped.len(), 1);
    assert_eq!(stale.dropped[0].command, other_batch.command);
    assert!(stale.blocked.is_none());
    assert_eq!(owner.pending_len(), 0);
}
