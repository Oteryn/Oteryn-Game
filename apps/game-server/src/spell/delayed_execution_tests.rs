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
    // D1218 read-only metadata: warnings/other session/epoch cannot substitute expiry.
    let expiry = super::super::companion_lifecycle::SavedFamiliarTimer {
        binding: binding.clone(),
        creature: target,
        lifecycle_epoch: 7,
        event: super::super::companion_lifecycle::FamiliarTimerEvent::Expiry,
    };
    let due = SemanticTimeMicros::from_micros(1_500_000);
    let pending = Pending {
        occurrence: SpellTimerOccurrence {
            command: binding.command,
            phase: 0,
        },
        due,
        payload: TimerPayload::Familiar(expiry.clone()),
    };
    assert_eq!(owner.familiar_expiry_due(caster, session, target, 7), None);
    owner.payloads.push(pending.clone());
    assert_eq!(
        owner.familiar_expiry_due(caster, session, target, 7),
        Some(due)
    );
    // #2199 Codex P2 4302737307: the owner cadence reads only this owner's due expiries.
    let before = SemanticTimeMicros::from_micros(1_499_999);
    assert!(
        owner
            .due_familiar_expiries(caster, session, before)
            .is_empty()
    );
    assert_eq!(
        owner.due_familiar_expiries(caster, session, due),
        vec![(target, 7)]
    );
    assert!(
        owner
            .due_familiar_expiries(other_caster, session, due)
            .is_empty()
    );
    assert!(
        owner
            .due_familiar_expiries(caster, other_session, due)
            .is_empty()
    );
    assert_eq!(
        owner.familiar_expiry_due(other_caster, session, target, 7),
        None
    );
    assert_eq!(
        owner.familiar_expiry_due(caster, other_session, target, 7),
        None
    );
    assert_eq!(owner.familiar_expiry_due(caster, session, caster, 7), None);
    assert_eq!(owner.familiar_expiry_due(caster, session, target, 8), None);
    let mut warning = expiry;
    warning.event = super::super::companion_lifecycle::FamiliarTimerEvent::Warning { index: 0 };
    owner.payloads.push(Pending {
        payload: TimerPayload::Familiar(warning),
        ..pending.clone()
    });
    assert_eq!(
        owner.familiar_expiry_due(caster, session, target, 7),
        Some(due)
    );
    owner.payloads.push(pending);
    assert_eq!(owner.familiar_expiry_due(caster, session, target, 7), None);
    // A warning is never an expiry, and one lifecycle is reported once.
    assert_eq!(
        owner.due_familiar_expiries(caster, session, due),
        vec![(target, 7)]
    );
    owner.payloads.clear();
    let original = OwnerCombatBatch {
        caster,
        attacker,
        current_lease_generation: 1,
        command: binding.command,
        occurrence: binding.occurrence.clone().into(),
        binding: binding.parent_binding.clone(),
        anchor: None,
        now_us: None,
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
            now_us: Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get())),
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

/// #2199 Codex P2 4302041461: a stored familiar return is one entry per actor/session, due
/// from its scheduled owner time, moved (not duplicated) by a retry backoff and gone once
/// cleared; it never touches another actor's entry.
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[test]
fn stored_familiar_return_is_one_due_entry_per_actor() {
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
    let actor = runtime.commit_fresh_session(admission).unwrap();
    let other_session = GameSessionId::decode(&id(93)).unwrap();
    let admission = runtime.reserve_fresh_session(other_session).unwrap();
    let other = runtime.commit_fresh_session(admission).unwrap();
    let mut owner = SpellTimerOwner::new(
        RuntimeScopeRefV1::channel(runtime.binding().world_id(), runtime.binding().channel_id()),
        runtime.binding().scope_generation(),
    )
    .unwrap();
    let spell = std::num::NonZeroU32::new(4).unwrap();
    assert_eq!(owner.familiar_return_due(actor, session, u64::MAX), None);
    assert!(owner.schedule_familiar_return(actor, session, spell, 1_000));
    assert_eq!(owner.familiar_return_due(actor, session, 999), None);
    assert_eq!(
        owner.familiar_return_due(actor, session, 1_000),
        Some(spell)
    );
    // Only the exact actor/session owns the entry.
    assert_eq!(owner.familiar_return_due(other, other_session, 1_000), None);
    assert_eq!(owner.familiar_return_due(actor, other_session, 1_000), None);
    // A failed attempt moves the same entry later; it is never doubled.
    assert!(owner.schedule_familiar_return(actor, session, spell, 11_000));
    assert_eq!(owner.familiar_return_due(actor, session, 10_999), None);
    assert_eq!(
        owner.familiar_return_due(actor, session, 11_000),
        Some(spell)
    );
    assert!(owner.schedule_familiar_return(other, other_session, spell, 0));
    let mut actors = owner.familiar_return_actors();
    actors.sort_unstable_by_key(|(a, _)| a.placement_identity());
    let mut expected = vec![(actor, session), (other, other_session)];
    expected.sort_unstable_by_key(|(a, _)| a.placement_identity());
    assert_eq!(actors, expected);
    // A completed (or no longer admissible) return clears only its own entry.
    owner.clear_familiar_return(actor, session);
    assert_eq!(owner.familiar_return_due(actor, session, u64::MAX), None);
    assert_eq!(owner.familiar_return_actors(), vec![(other, other_session)]);
}

/// #2199 Codex P2 4302737307: a due familiar expiry is no despawn in the timer lane. The lane
/// hands it to the owner cadence (`due_familiar_expiries`) and keeps its payload, so the
/// fenced lifecycle writer closes the durable row before it removes the familiar; once the
/// familiar is gone, the lane drops the payload.
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[test]
fn due_familiar_expiry_is_handed_to_the_owner_cadence_without_a_despawn() {
    use crate::foundation::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};
    use crate::spell::companion_lifecycle::{
        FamiliarTimerEvent, SavedFamiliarTimer, commit_familiar_timer, stage_familiar_timer,
    };
    struct Clock(u64);
    impl OwnerClock for Clock {
        fn now(&self) -> SemanticTimeMicros {
            SemanticTimeMicros::from_micros(self.0)
        }
    }
    let (mut runtime, caster, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(97);
    runtime.initialize_first_entry_position(caster).unwrap();
    // A typed physical owner fixture, not an active source importer or grant.
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![CompiledCreaturePolicy {
                    definition_key: "creature:knight/familiar".into(),
                    definition_revision: "r20".into(),
                    display_name: "Knight familiar".into(),
                    maximum_health: 812,
                    base_speed: 220,
                    outfit_look_type: 991,
                    object_look_type: None,
                    summonable: false,
                    convinceable: false,
                    mana_cost: None,
                    is_familiar: true,
                    condition_immunities: Vec::new(),
                    preferred_distance: Some(1),
                    reward_boss: Some(false),
                    defense: Some(1),
                    armor: Some(10),
                    mitigation: None,
                    resistances: Vec::new(),
                    damage_immunities: Vec::new(),
                    healing_from_damage: Vec::new(),
                    flags: CreatureFlags {
                        attackable: true,
                        illusionable: false,
                        health_hidden: false,
                    },
                }],
            )
            .unwrap(),
        )
        .unwrap();
    let familiar = runtime
        .create_companion(
            caster,
            session,
            "creature:knight/familiar",
            MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
            None,
            Some(1_800_001_800),
            7,
        )
        .unwrap();
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .unwrap();
    let row = document["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Knight familiar")
        .unwrap();
    let spell = super::super::native::spell_from_bundle(
        &serde_json::json!({"spell":row["spell"]}),
        &row["dependencies"],
    )
    .unwrap();
    let mut character = *session.as_bytes();
    character[15] = 98;
    let binding = CastBinding {
        spell,
        caster,
        attacker: CharacterId::decode(&character).unwrap(),
        command: CommandRef::new(session, CommandId::new(1).unwrap()),
        occurrence: AbilityOccurrence::new(
            "spell-timer:familiar-expiry",
            crate::ability::RevisionSet::new(
                "rules:1",
                "content:1",
                "world:1",
                "formula:1",
                "simulation:1",
            )
            .unwrap(),
        )
        .unwrap(),
        parent_binding: b"qualified-cast:knight-familiar".to_vec(),
        cast_at: SemanticTimeMicros::from_micros(0),
        cast_position: TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        cast_snapshot: None,
    };
    let saved = SavedFamiliarTimer {
        binding: binding.clone(),
        creature: familiar,
        lifecycle_epoch: 7,
        event: FamiliarTimerEvent::Expiry,
    };
    // The lane's own commit never removes an expired familiar.
    let staged = stage_familiar_timer(&runtime, &saved).unwrap().unwrap();
    assert!(commit_familiar_timer(&mut runtime, staged).is_err());
    assert!(runtime.companion_snapshot(familiar).is_ok());
    let mut owner = SpellTimerOwner::new(
        RuntimeScopeRefV1::channel(runtime.binding().world_id(), runtime.binding().channel_id()),
        runtime.binding().scope_generation(),
    )
    .unwrap();
    let occurrence = SpellTimerOccurrence {
        command: binding.command,
        phase: 0,
    };
    let due = SemanticTimeMicros::from_micros(900_000_000);
    let payload = TimerPayload::Familiar(saved);
    let stamp = runtime.issue_owner_work().unwrap();
    owner
        .lane
        .schedule(
            runtime.owner_fence().unwrap(),
            stamp,
            payload.family(),
            OwnerSpellTimerKey::Cast(occurrence),
            payload.target(),
            due,
        )
        .unwrap();
    owner.payloads.push(Pending {
        occurrence,
        due,
        payload,
    });
    let mut build = |_: &ChannelRuntimeV1,
                     _: &TimerPayload,
                     _: SpellTimerOccurrence,
                     _: SemanticTimeMicros,
                     _: RuntimeWorkStamp| Err(Error::InvalidPayload);
    assert!(
        owner
            .due_familiar_expiries(caster, session, SemanticTimeMicros::from_micros(0))
            .is_empty()
    );
    for at in [due.get(), due.get() + 1_000_000] {
        let stamp = runtime.issue_owner_work().unwrap();
        let fired = owner
            .fire_due_current(&mut runtime, &Clock(at), stamp, &mut build)
            .unwrap();
        assert!(fired.blocked.is_none());
        assert!(fired.dropped.is_empty());
        assert!(fired.familiar_receipts.is_empty());
        assert!(runtime.companion_snapshot(familiar).is_ok());
        assert_eq!(
            owner.due_familiar_expiries(caster, session, SemanticTimeMicros::from_micros(at)),
            vec![(familiar, 7)]
        );
        assert_eq!(
            owner.familiar_expiry_due(caster, session, familiar, 7),
            Some(due)
        );
    }
    // Stand-in for the lifecycle writer's removal after its durable commit.
    let live = runtime.companion_snapshot(familiar).unwrap();
    runtime.despawn_companion(caster, session, &live).unwrap();
    let stamp = runtime.issue_owner_work().unwrap();
    let fired = owner
        .fire_due_current(
            &mut runtime,
            &Clock(due.get() + 2_000_000),
            stamp,
            &mut build,
        )
        .unwrap();
    assert_eq!(fired.dropped, vec![occurrence]);
    assert!(owner.due_familiar_expiries(caster, session, due).is_empty());
}
