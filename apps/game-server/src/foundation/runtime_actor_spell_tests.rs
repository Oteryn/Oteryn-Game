use super::super::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellAnchor, SpellOccurrenceBinding,
};
use super::*;

fn uuid(tag: u8) -> [u8; 16] {
    [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}
fn position(x: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y: 10, floor: 7 }
}

struct Fixture {
    runtime: ChannelRuntimeV1,
    caster: ExactActorRef,
    targets: Vec<ExactActorRef>,
    session: GameSessionId,
}
fn fixture(generation: u64) -> Fixture {
    fixture_with_position(generation, false)
}
fn fixture_with_position(generation: u64, at_entry: bool) -> Fixture {
    let world = WorldId::decode(&uuid(0x40)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(0x41)).expect("channel"),
        NodeId::decode(&uuid(0x42)).expect("node"),
        1,
        generation,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .expect("real runtime");
    let session = GameSessionId::decode(&uuid(0x21)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let caster = runtime.commit_fresh_session(reservation).expect("commit");
    if at_entry {
        runtime
            .initialize_first_entry_position(caster)
            .expect("actual pinned entry");
    } else {
        runtime
            .initialize_movement_test_position(caster, position(10))
            .expect("position");
    }
    let targets = (11..14)
        .map(|x| {
            (if at_entry {
                runtime.admit_pinned_test_creature(position(x))
            } else {
                runtime.admit_test_creature(position(x))
            })
            .expect("real creature")
        })
        .collect();
    Fixture {
        runtime,
        caster,
        targets,
        session,
    }
}
fn batch(f: &Fixture, sequence: u64, effects: Vec<OwnerCombatEffect>) -> OwnerCombatBatch {
    OwnerCombatBatch {
        caster: f.caster,
        attacker: CharacterId::decode(&uuid(0x22)).expect("character"),
        current_lease_generation: 1,
        command: CommandRef::new(
            f.session,
            super::super::super::CommandId::new(sequence).expect("command"),
        ),
        occurrence: SpellOccurrenceBinding {
            id: "spell-batch:qualified".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"source-qualified:recipe-sha256".to_vec(),
        anchor: None,
        now_ms: 100,
        effects,
        deferred: None,
    }
}
fn damage(target: ExactActorRef, ordinal: u16, magnitude: i64) -> OwnerCombatEffect {
    OwnerCombatEffect {
        target,
        sub_ordinal: ordinal,
        change: OwnerCombatChange::Damage {
            target_atom: "test:creature".into(),
            magnitude,
        },
    }
}
fn health(f: &Fixture, index: usize) -> i64 {
    f.runtime
        .creature_combat_facts(f.targets[index], 100)
        .expect("physical facts")
        .health
}

#[test]
fn three_effect_candidate_stages_real_slots_and_one_commit_installs_all() {
    let mut f = fixture(1);
    let b = batch(
        &f,
        1,
        f.targets
            .iter()
            .enumerate()
            .map(|(i, target)| damage(*target, i as u16, 3))
            .collect(),
    );
    let staged = f
        .runtime
        .stage_spell_batch(&b)
        .expect("candidate stage beyond old Ability cap");
    assert_eq!((health(&f, 0), health(&f, 1), health(&f, 2)), (20, 20, 20));
    let receipt = f
        .runtime
        .commit_spell_batch(staged)
        .expect("atomic physical replacement");
    assert!(receipt.applied);
    assert_eq!(receipt.effects.len(), 3);
    assert_eq!((health(&f, 0), health(&f, 1), health(&f, 2)), (17, 17, 17));
    let replay = f
        .runtime
        .stage_spell_batch(&b)
        .expect("whole-command replay");
    assert!(
        !f.runtime
            .commit_spell_batch(replay)
            .expect("original receipt")
            .applied
    );
    assert_eq!(health(&f, 0), 17);
}

#[test]
fn original_typed_batch_retention_survives_lethal_targets_and_is_not_an_alias() {
    let mut f = fixture(1);
    // Nonzero global hit ordinal is still whole-cast phase0, not a deferred timer phase.
    let original = batch(&f, 1, vec![damage(f.targets[0], 7, 20)]);
    assert!(
        f.runtime
            .retained_spell_batch(f.caster, original.attacker, 1, original.command)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.runtime.creature_spell_target_atom(f.targets[0]).unwrap(),
        "test:creature"
    );
    let staged = f.runtime.stage_spell_batch(&original).unwrap();
    assert!(f.runtime.commit_spell_batch(staged).unwrap().applied);
    let mut retained = f
        .runtime
        .retained_spell_batch(f.caster, original.attacker, 1, original.command)
        .unwrap()
        .unwrap();
    assert_eq!(retained, original);
    // Caller data cannot edit the physical owner's retained canonical input.
    retained.binding.push(b'!');
    assert!(matches!(
        f.runtime.stage_spell_batch(&retained),
        Err(Error::CommandConflict)
    ));
    let retained = f
        .runtime
        .retained_spell_batch(f.caster, original.attacker, 1, original.command)
        .unwrap()
        .unwrap();
    assert_eq!(retained, original);
    let replay = f.runtime.stage_spell_batch(&retained).unwrap();
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    assert!(
        f.runtime
            .retained_spell_batch(
                f.caster,
                CharacterId::decode(&uuid(0x24)).unwrap(),
                1,
                original.command
            )
            .is_err()
    );
    assert!(
        f.runtime
            .retained_spell_batch(f.caster, original.attacker, 2, original.command)
            .is_err()
    );
}

#[test]
fn failed_last_effect_does_not_write_earlier_staged_damage_or_caster_receipt() {
    let mut f = fixture(1);
    let mut b = batch(
        &f,
        1,
        vec![damage(f.targets[0], 0, 3), damage(f.targets[1], 1, -1)],
    );
    assert!(matches!(
        f.runtime.stage_spell_batch(&b),
        Err(Error::Owner(CarrierError::InvalidDamage))
    ));
    assert_eq!((health(&f, 0), health(&f, 1)), (20, 20));
    b.effects[1] = damage(f.targets[1], 1, 3);
    let staged = f
        .runtime
        .stage_spell_batch(&b)
        .expect("same command still uncommitted");
    assert!(
        f.runtime
            .commit_spell_batch(staged)
            .expect("commit")
            .applied
    );
    assert_eq!((health(&f, 0), health(&f, 1)), (17, 17));
}

#[test]
fn change_between_stage_and_commit_rejects_entire_replacement_group() {
    let mut f = fixture(1);
    let b = batch(
        &f,
        1,
        vec![damage(f.targets[0], 0, 3), damage(f.targets[1], 1, 3)],
    );
    let staged = f.runtime.stage_spell_batch(&b).expect("stage");
    f.runtime
        .validate_staged_spell_batch(&staged)
        .expect("actual slots still unchanged before durable work");
    f.runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            f.targets[0],
            OwnerDamageCommand {
                target: b"test:creature",
                occurrence: b"independent-owner-hit",
                binding: b"independent-owner-hit\0qualified",
                damage: 7,
            },
        )
        .expect("independent actual mutation");
    assert_eq!(
        f.runtime.validate_staged_spell_batch(&staged),
        Err(Error::SnapshotChanged)
    );
    assert_eq!((health(&f, 0), health(&f, 1)), (13, 20));
    assert_eq!(
        f.runtime.commit_spell_batch(staged),
        Err(Error::SnapshotChanged)
    );
    assert_eq!((health(&f, 0), health(&f, 1)), (13, 20));
}

#[test]
fn pending_source_reserves_all_actual_touched_slots_and_only_original_batch_can_finish() {
    let mut f = fixture(1);
    let original = batch(
        &f,
        1,
        vec![damage(f.targets[0], 0, 3), damage(f.targets[1], 1, 4)],
    );
    let mut staged = f.runtime.stage_spell_batch(&original).unwrap();
    f.runtime.reserve_spell_batch(&mut staged).unwrap();
    for actor in [f.caster, f.targets[0], f.targets[1]] {
        assert!(f.runtime.actor_spell_reserved(actor));
        assert_eq!(
            f.runtime.assert_actor_spell_unreserved(actor),
            Err(CarrierError::PlanConflict)
        );
    }
    assert!(!f.runtime.actor_spell_reserved(f.targets[2]));
    // An unrelated actual creature can change during a retained unknown outcome.
    // The original source stage compares only affected reserved slots, not a global census.
    f.runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            f.targets[2],
            OwnerDamageCommand {
                target: b"test:creature",
                occurrence: b"independent-unrelated-owner-hit",
                binding: b"independent-unrelated-owner-hit\0qualified",
                damage: 5,
            },
        )
        .expect("unrelated actual mutation remains available");
    f.runtime
        .validate_staged_spell_batch(&staged)
        .expect("affected originals still exact");
    assert_eq!(health(&f, 2), 15);
    let competing = batch(&f, 2, vec![damage(f.targets[0], 0, 1)]);
    assert!(matches!(
        f.runtime.stage_spell_batch(&competing),
        Err(Error::SnapshotChanged)
    ));
    assert_eq!((health(&f, 0), health(&f, 1)), (20, 20));
    // Dropping and rebuilding a data proof retains actual owner reservations.
    drop(staged);
    let retry = f.runtime.stage_spell_batch(&original).unwrap();
    assert!(f.runtime.commit_spell_batch(retry).unwrap().applied);
    assert_eq!((health(&f, 0), health(&f, 1), health(&f, 2)), (17, 16, 15));
    for actor in [f.caster, f.targets[0], f.targets[1]] {
        assert!(!f.runtime.actor_spell_reserved(actor));
    }
    let replay = f.runtime.stage_spell_batch(&original).unwrap();
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    let mut stale = f.targets[0];
    stale.0.actor_local_generation = ActorLocalGeneration(stale.0.actor_local_generation.0 + 1);
    assert!(f.runtime.actor_spell_reserved(stale));
    assert_eq!(
        f.runtime.assert_actor_spell_unreserved(stale),
        Err(CarrierError::StaleActorGeneration)
    );
}

#[test]
fn same_command_cannot_change_target_magnitude_source_or_payment() {
    let mut f = fixture(1);
    let b = batch(&f, 1, vec![damage(f.targets[0], 0, 3)]);
    let staged = f.runtime.stage_spell_batch(&b).expect("stage");
    f.runtime.commit_spell_batch(staged).expect("commit");
    for change in 0..4 {
        let mut changed = b.clone();
        match change {
            0 => changed.effects[0] = damage(f.targets[0], 0, 4),
            1 => changed.effects[0].target = f.targets[1],
            2 => changed.binding.push(99),
            _ => {
                changed.anchor = Some(SpellAnchor {
                    expected_revision: 1,
                    next_revision: 2,
                    paid_mana: 50,
                    paid_soul: 0,
                    cooldown_deadlines: vec![],
                })
            }
        }
        assert!(matches!(
            f.runtime.stage_spell_batch(&changed),
            Err(Error::CommandConflict)
        ));
    }
    assert_eq!((health(&f, 0), health(&f, 1)), (17, 20));
}

#[test]
fn actual_creature_heal_uses_authored_maximum_and_replays_without_more_health() {
    let mut f = fixture(1);
    let b = batch(&f, 1, vec![damage(f.targets[0], 0, 9)]);
    let staged = f.runtime.stage_spell_batch(&b).expect("stage");
    f.runtime.commit_spell_batch(staged).expect("damage");
    let heal = batch(
        &f,
        2,
        vec![OwnerCombatEffect {
            target: f.targets[0],
            sub_ordinal: 0,
            change: OwnerCombatChange::Heal {
                target_atom: "test:creature".into(),
                magnitude: 100,
            },
        }],
    );
    let staged = f.runtime.stage_spell_batch(&heal).expect("heal stage");
    let receipt = f.runtime.commit_spell_batch(staged).expect("heal");
    let hp = receipt.effects[0].health.expect("actual HP receipt");
    assert_eq!((hp.health_before, hp.health_after), (11, 20));
    assert_eq!(
        f.runtime
            .creature_combat_facts(f.targets[0], 100)
            .expect("max")
            .maximum_health,
        20
    );
    let replay = f.runtime.stage_spell_batch(&heal).expect("replay");
    assert!(
        !f.runtime
            .commit_spell_batch(replay)
            .expect("no extra heal")
            .applied
    );
}

#[test]
fn actor_ai_override_is_physical_state_and_deadlines_gate_actual_owner_reads() {
    let mut f = fixture(1);
    let b = batch(
        &f,
        1,
        vec![OwnerCombatEffect {
            target: f.targets[0],
            sub_ordinal: 0,
            change: OwnerCombatChange::MonsterAi(MonsterAiState {
                forced_distance: Some((1, 8100)),
                challenged_to: Some((f.caster, 6100)),
            }),
        }],
    );
    let staged = f.runtime.stage_spell_batch(&b).expect("AI stage");
    f.runtime.commit_spell_batch(staged).expect("AI commit");
    let active = f
        .runtime
        .creature_combat_facts(f.targets[0], 6099)
        .expect("active")
        .monster_ai
        .expect("AI");
    assert_eq!(active.challenged_to, Some((f.caster, 6100)));
    let expired = f
        .runtime
        .creature_combat_facts(f.targets[0], 6100)
        .expect("expiry")
        .monster_ai
        .expect("AI");
    assert_eq!(expired.challenged_to, None);
    assert_eq!(expired.forced_distance, Some((1, 8100)));
}

#[test]
fn player_health_and_anchor_require_the_real_player_owner_preflight() {
    let mut f = fixture(1);
    let b = batch(
        &f,
        1,
        vec![OwnerCombatEffect {
            target: f.caster,
            sub_ordinal: 0,
            change: OwnerCombatChange::Heal {
                target_atom: "actor:player-owner".to_owned(),
                magnitude: 7,
            },
        }],
    );
    let staged = f
        .runtime
        .stage_spell_batch(&b)
        .expect("route existing player owner");
    assert_eq!(staged.player_effects().len(), 1);
    assert_eq!(
        f.runtime.commit_spell_batch(staged),
        Err(Error::PlayerVitalsOwnerRequired)
    );
    let real = batch(&f, 1, vec![damage(f.targets[0], 0, 3)]);
    let staged = f
        .runtime
        .stage_spell_batch(&real)
        .expect("no phantom consumed-command receipt");
    assert_eq!(
        f.runtime
            .commit_spell_batch(staged)
            .expect("actual commit")
            .effects[0]
            .health
            .expect("HP")
            .health_before,
        20
    );
}

#[test]
fn stale_target_and_replaced_namespace_reject_without_any_slot_mutation() {
    let mut f = fixture(1);
    let b = batch(
        &f,
        1,
        vec![damage(f.targets[0], 0, 3), damage(f.targets[1], 1, 3)],
    );
    let staged = f.runtime.stage_spell_batch(&b).expect("stage");
    f.runtime.remove_test_actor(f.targets[1]).expect("despawn");
    assert_eq!(
        f.runtime.commit_spell_batch(staged),
        Err(Error::SnapshotChanged)
    );
    assert_eq!(health(&f, 0), 20);
    let newer = fixture(2);
    assert!(matches!(
        newer.runtime.stage_spell_batch(&b),
        Err(Error::Owner(CarrierError::WrongScope))
    ));
}

#[test]
fn player_session_lookup_requires_actual_current_control_and_pinned_position_context() {
    let mut f = fixture_with_position(1, true);
    let (actor, position) = f
        .runtime
        .positioned_player_for_session(f.session)
        .expect("owner read")
        .expect("committed actual entry");
    assert_eq!(actor, f.caster);
    assert_eq!(position.context(), f.runtime.pinned_movement_context());
    let absent = GameSessionId::decode(&uuid(0x25)).expect("different session");
    assert_eq!(
        f.runtime
            .positioned_player_for_session(absent)
            .expect("bounded lookup"),
        None
    );
    f.runtime
        .record_control_loss(
            f.caster,
            f.session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 500,
            },
        )
        .expect("actual durable loss mirror");
    assert_eq!(
        f.runtime
            .positioned_player_for_session(f.session)
            .expect("uncontrolled read"),
        None
    );
    let wrong_context = fixture(1);
    assert_eq!(
        wrong_context
            .runtime
            .positioned_player_for_session(wrong_context.session),
        Err(CarrierError::PositionContextMismatch)
    );
}

#[test]
fn candidate_ordinal_ceiling_and_evicted_command_replay_fail_before_mutation() {
    let mut f = fixture(1);
    let too_far = batch(&f, 1, vec![damage(f.targets[0], MAX_EFFECTS as u16, 1)]);
    assert!(matches!(
        f.runtime.stage_spell_batch(&too_far),
        Err(Error::InvalidBatch)
    ));
    assert_eq!(health(&f, 0), 20);
    for sequence in 1..=17 {
        let b = batch(&f, sequence, vec![damage(f.targets[0], 255, 1)]);
        let staged = f
            .runtime
            .stage_spell_batch(&b)
            .expect("last legal candidate ordinal");
        f.runtime
            .commit_spell_batch(staged)
            .expect("physical damage");
    }
    assert_eq!(health(&f, 0), 3);
    let evicted = batch(&f, 1, vec![damage(f.targets[0], 255, 1)]);
    assert!(matches!(
        f.runtime.stage_spell_batch(&evicted),
        Err(Error::StaleCommand)
    ));
    assert_eq!(health(&f, 0), 3);
}

#[test]
fn committed_timer_phase_survives_control_loss_but_new_or_substituted_input_does_not() {
    use super::super::runtime_actor_spell_types::DeferredCommitAuthority;
    let mut f = fixture(1);
    let original = batch(&f, 1, vec![damage(f.targets[0], 0, 2)]);
    let staged = f
        .runtime
        .stage_spell_batch(&original)
        .expect("original cast");
    assert!(f.runtime.commit_spell_batch(staged).unwrap().applied);
    assert_eq!(health(&f, 0), 18);

    let mut due = original.clone();
    due.now_ms = 200;
    due.binding = b"source-qualified:recipe-sha256:due-phase-1".to_vec();
    due.effects = vec![damage(f.targets[0], 1, 2)];
    let stamp = f
        .runtime
        .issue_owner_work()
        .expect("real current owner work");
    let binding = f.runtime.binding();
    let scope =
        super::super::super::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id());
    due.deferred = Some(
        DeferredCommitAuthority::from_due(
            f.runtime.owner_fence().unwrap(),
            scope,
            binding.scope_generation(),
            stamp,
            1,
            1,
            original.binding.clone(),
            &due,
        )
        .expect("owner-issued immutable deferred token"),
    );
    f.runtime
        .record_control_loss(
            f.caster,
            f.session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 500,
            },
        )
        .expect("actual same-actor loss");

    let fresh = batch(&f, 2, vec![damage(f.targets[0], 0, 3)]);
    assert!(matches!(
        f.runtime.stage_spell_batch(&fresh),
        Err(Error::InvalidBatch)
    ));
    let mut unsealed = due.clone();
    unsealed.deferred = None;
    assert!(matches!(
        f.runtime.stage_spell_batch(&unsealed),
        Err(Error::InvalidBatch)
    ));
    let mut substituted = due.clone();
    substituted.effects[0] = damage(f.targets[0], 1, 3);
    assert!(matches!(
        f.runtime.stage_spell_batch(&substituted),
        Err(Error::InvalidBatch)
    ));
    let mut zero_phase = unsealed.clone();
    zero_phase.deferred = Some(
        DeferredCommitAuthority::from_due(
            f.runtime.owner_fence().unwrap(),
            scope,
            binding.scope_generation(),
            stamp,
            0,
            1,
            original.binding.clone(),
            &zero_phase,
        )
        .unwrap(),
    );
    assert!(matches!(
        f.runtime.stage_spell_batch(&zero_phase),
        Err(Error::InvalidBatch)
    ));
    let mut foreign_session = due.clone();
    foreign_session.command = CommandRef::new(
        GameSessionId::decode(&uuid(0x25)).unwrap(),
        foreign_session.command.command_id(),
    );
    assert!(f.runtime.stage_spell_batch(&foreign_session).is_err());
    assert_eq!(health(&f, 0), 18);

    let staged = f
        .runtime
        .stage_spell_batch(&due)
        .expect("already committed timer history");
    assert!(f.runtime.commit_spell_batch(staged).unwrap().applied);
    assert_eq!(health(&f, 0), 16);
    let replay = f
        .runtime
        .stage_spell_batch(&due)
        .expect("original deferred receipt");
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    assert_eq!(health(&f, 0), 16);
    assert!(
        f.runtime
            .player_control_facts(f.caster, f.session)
            .unwrap()
            .control_loss
            .is_some()
    );

    f.runtime
        .restore_control_after_committed_reentry(f.caster, f.session, 1, 1_000)
        .expect("same-actor committed reentry");
    assert_eq!(
        f.runtime
            .current_player_reentry_protection(f.caster, f.session, 1_000),
        Ok(true)
    );
    let replay = f
        .runtime
        .stage_spell_batch(&due)
        .expect("retry during reentry protection");
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    f.runtime.retire_owner_cycle();
    assert!(f.runtime.stage_spell_batch(&due).is_err());
}

#[test]
fn real_assignment_owner_issues_scoped_work_and_retirement_refuses_staged_mutation() {
    let mut f = fixture(1);
    let stamp = f
        .runtime
        .issue_owner_work()
        .expect("independently committed assignment");
    assert!(
        f.runtime
            .owner_fence()
            .expect("current actual owner")
            .accepts_stamp(stamp)
    );
    let other_world = WorldId::decode(&uuid(0x50)).expect("other world");
    let mut other = ChannelRuntimeV1::from_committed_assignment(
        other_world,
        ChannelId::decode(&uuid(0x51)).expect("other channel"),
        NodeId::decode(&uuid(0x52)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(other_world),
    )
    .expect("independent same-generation owner");
    let foreign_stamp = other.issue_owner_work().expect("foreign owner work");
    assert!(
        !f.runtime
            .owner_fence()
            .expect("own current fence")
            .accepts_stamp(foreign_stamp)
    );
    let b = batch(&f, 1, vec![damage(f.targets[0], 0, 3)]);
    let staged = f
        .runtime
        .stage_spell_batch(&b)
        .expect("before actual handoff");
    f.runtime.retire_owner_cycle();
    assert!(f.runtime.owner_fence().is_err());
    assert!(f.runtime.issue_owner_work().is_err());
    assert!(f.runtime.commit_spell_batch(staged).is_err());
    // This is a structural receipt check only: every public owner read is now
    // denied by the retired independent continuity authority.
    assert!(matches!(
        &f.runtime.carrier.slots[1],
        Slot::CreatureOccupied { health: 20, .. }
    ));
    assert!(!f.runtime.owner_cycle.accepts_stamp(stamp));
}

#[test]
fn facing_is_unknown_until_real_cardinal_commit_and_failed_compare_does_not_turn_actor() {
    let mut f = fixture_with_position(1, true);
    let (_, original) = f
        .runtime
        .positioned_player_for_session(f.session)
        .expect("actual owner roster")
        .expect("pinned entry");
    assert_eq!(original.facing(), None);
    let p = original.position();
    let east = f
        .runtime
        .borrow_movement_position()
        .commit_cardinal(original, MovementLocalPosition { x: p.x + 1, ..p })
        .expect("real cardinal position owner");
    assert_eq!(east.facing(), Some(MovementFacing::East));
    assert!(
        f.runtime
            .borrow_movement_position()
            .commit_cardinal(original, MovementLocalPosition { y: p.y - 1, ..p })
            .is_err()
    );
    let (_, current) = f
        .runtime
        .positioned_player_for_session(f.session)
        .expect("current owner")
        .expect("current actor");
    assert_eq!(current, east);
    let north = f
        .runtime
        .borrow_movement_position()
        .commit_cardinal(
            current,
            MovementLocalPosition {
                y: current.position().y - 1,
                ..current.position()
            },
        )
        .expect("real northward write");
    assert_eq!(north.facing(), Some(MovementFacing::North));
}

#[test]
#[allow(dead_code)]
fn accepted_d140_layout_receipt_is_separate_from_the_local_spell_candidate_budget() {
    // Exact accepted HEAD representation, deliberately excluding new candidate
    // fields. This is a layout-only receipt, never another runtime HP carrier.
    struct AcceptedPosition {
        actor_local_id: ActorLocalId,
        actor_local_generation: ActorLocalGeneration,
        context: PreProductionPositionContext,
        position: LocalPosition,
        revision: u64,
    }
    enum AcceptedSlot {
        VacantReusable {
            generation: u64,
            next_free: Option<u32>,
        },
        Occupied {
            generation: u64,
            actor: ActorState,
            game_session_id: Option<GameSessionId>,
            committed: bool,
            position: Option<AcceptedPosition>,
            control_loss: Option<ControlLossMark>,
        },
        CreatureOccupied {
            generation: u64,
            actor: ActorState,
            position: Option<AcceptedPosition>,
            target_identity: Arc<[u8]>,
            health: i64,
            committed: Box<DamageReceipts>,
            damage_contributors: Box<DamageContributors>,
        },
        Exhausted {
            generation: u64,
        },
    }
    assert_eq!(
        std::mem::size_of::<AcceptedSlot>(),
        168,
        "accepted D140 representation"
    );
    assert_eq!(
        std::mem::size_of::<Slot>(),
        // D314: measured on the merged tree with COND-1c's boxed lifecycle; budget 184.
        176,
        "explicit local SPELL-BATCH candidate"
    );
    assert_eq!(
        176 * 32,
        5632,
        "candidate 32-slot inline storage; heap state separately bounded"
    );
}

#[test]
fn source_metadata_touches_reserve_real_slots_without_synthesized_combat_effects() {
    use super::super::runtime_actor_companion::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    let mut f = fixture_with_position(1, true);
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:rat".into(),
        definition_revision: "test:qualified:1".into(),
        display_name: "Rat".into(),
        maximum_health: 20,
        base_speed: 100,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: true,
        convinceable: true,
        mana_cost: Some(200),
        is_familiar: false,
        condition_immunities: vec![],
        armor: Some(0),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        preferred_distance: Some(1),
        reward_boss: Some(false),
        flags: CreatureFlags {
            attackable: true,
            illusionable: true,
            health_hidden: false,
        },
    };
    f.runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy]).unwrap(),
        )
        .unwrap();
    f.runtime
        .install_creature_policy(f.targets[0], "creature:rat")
        .unwrap();
    let snapshot = f.runtime.companion_snapshot(f.targets[0]).unwrap();
    let mut original = batch(&f, 1, vec![]);
    original.anchor = Some(SpellAnchor {
        expected_revision: 1,
        next_revision: 2,
        paid_mana: 0,
        paid_soul: 0,
        cooldown_deadlines: vec![],
    });
    original.binding = b"{\"source\":\"qualified-familiar-metadata\"}".to_vec();
    let touches = QualifiedCompanionTouches::bind(&mut original, &[snapshot.clone()]).unwrap();
    let mut staged = f
        .runtime
        .stage_spell_batch_with_companion_touches(&original, &touches)
        .unwrap();
    assert!(staged.player_effects().is_empty());
    f.runtime.reserve_spell_batch(&mut staged).unwrap();
    assert!(f.runtime.actor_spell_reserved(f.targets[0]));
    assert_eq!(
        f.runtime.mark_companion_party_protection(&snapshot),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(health(&f, 0), 20);
    // Retention and rebuilding preserve the exact actual slot reservation, with no zero hit.
    drop(staged);
    let mut staged = f
        .runtime
        .stage_spell_batch_with_companion_touches(&original, &touches)
        .unwrap();
    f.runtime.reserve_spell_batch(&mut staged).unwrap();
    f.runtime
        .release_companion_touches_for_source_commit(&mut staged)
        .unwrap();
    assert!(!f.runtime.actor_spell_reserved(f.targets[0]));
    assert!(f.runtime.actor_spell_reserved(f.caster));
    f.runtime
        .mark_companion_party_protection(&snapshot)
        .unwrap();
    // Source metadata is no longer in unchanged replacements and cannot be reverted by payment.
    f.runtime.validate_staged_spell_batch(&staged).unwrap();
    assert!(
        f.runtime
            .companion_snapshot(f.targets[0])
            .unwrap()
            .state
            .party_protection
    );
    assert_eq!(health(&f, 0), 20);
    let mut changed = original.clone();
    changed.binding = b"{\"source_companion_touches\":\"forged\"}".to_vec();
    assert!(matches!(
        f.runtime
            .stage_spell_batch_with_companion_touches(&changed, &touches),
        Err(Error::CommandConflict)
    ));
    // This test stops at physical source staging: it makes no assertion of durable receipt authority.
}

#[test]
fn source_relocation_stages_same_slot_with_batch_and_reserves_destination_until_original_commit() {
    // This sealed fixture covers physical owner atomicity only. Production eligibility is
    // constructed exclusively by the World/SQL relocation owner, never these coordinates.
    struct Proof(
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    );
    impl relocation_seal::Sealed for Proof {}
    impl SpellRelocationProof for Proof {
        fn parts(
            &self,
        ) -> (
            ExactActorRef,
            MovementPositionSnapshot,
            MovementLocalPosition,
        ) {
            (self.0, self.1, self.2)
        }
    }
    let mut f = fixture_with_position(1, true);
    let expected = f.runtime.read_actor_position(f.caster).unwrap();
    let p = expected.position();
    let destination = MovementLocalPosition { x: p.x + 1, ..p };
    let mut original = batch(&f, 1, vec![damage(f.targets[0], 0, 3)]);
    original.binding = b"{\"source\":\"sealed-physical-relocation-test\"}".to_vec();
    let relocation =
        QualifiedSpellRelocation::bind(&mut original, Proof(f.caster, expected, destination))
            .unwrap();
    assert!(matches!(
        f.runtime.stage_spell_batch(&original),
        Err(Error::InvalidBatch)
    ));
    let mut staged = f
        .runtime
        .stage_spell_batch_with_relocation(&original, &relocation)
        .unwrap();
    f.runtime.reserve_spell_batch(&mut staged).unwrap();
    assert_eq!(f.runtime.read_actor_position(f.caster).unwrap(), expected);
    assert_eq!(health(&f, 0), 20);
    assert!(
        f.runtime
            .position_occupied_by_other(f.targets[2], destination)
            .unwrap()
    );
    drop(staged);
    let staged = f
        .runtime
        .stage_spell_batch_with_relocation(&original, &relocation)
        .unwrap();
    assert!(f.runtime.commit_spell_batch(staged).unwrap().applied);
    let current = f.runtime.read_actor_position(f.caster).unwrap();
    assert_eq!(current.position(), destination);
    assert_eq!(current.revision(), expected.revision() + 1);
    assert_eq!(current.facing(), expected.facing());
    assert_eq!(health(&f, 0), 17);
    let replay = f
        .runtime
        .stage_spell_batch_with_relocation(&original, &relocation)
        .unwrap();
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    assert_eq!(f.runtime.read_actor_position(f.caster).unwrap(), current);
}

fn master_assignment_fixture() -> Fixture {
    use super::super::runtime_actor_companion::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    let mut f = fixture_with_position(1, true);
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:rat".into(),
        definition_revision: "test:qualified:1".into(),
        display_name: "Rat".into(),
        maximum_health: 20,
        base_speed: 100,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: true,
        convinceable: true,
        mana_cost: Some(200),
        is_familiar: false,
        condition_immunities: vec![],
        armor: Some(0),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        preferred_distance: Some(1),
        reward_boss: Some(false),
        flags: CreatureFlags {
            attackable: true,
            illusionable: true,
            health_hidden: false,
        },
    };
    f.runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy]).unwrap(),
        )
        .unwrap();
    for actor in &f.targets {
        f.runtime
            .install_creature_policy(*actor, "creature:rat")
            .unwrap();
    }
    f
}

fn master_assignment_batch(f: &Fixture, sequence: u64) -> OwnerCombatBatch {
    let expected = f.runtime.companion_snapshot(f.targets[0]).unwrap();
    let assignment = f
        .runtime
        .prepare_companion_assignment(f.caster, f.session, &expected)
        .unwrap();
    batch(
        f,
        sequence,
        vec![OwnerCombatEffect {
            target: f.targets[0],
            sub_ordinal: 0,
            change: OwnerCombatChange::CompanionMaster(Box::new(assignment)),
        }],
    )
}

#[test]
fn actual_master_assignment_reserves_and_retries_without_early_install_or_second_assignment() {
    let mut f = master_assignment_fixture();
    let original = master_assignment_batch(&f, 1);
    let before = f.runtime.companion_snapshot(f.targets[0]).unwrap();
    let mut staged = f.runtime.stage_spell_batch(&original).unwrap();
    f.runtime.reserve_spell_batch(&mut staged).unwrap();
    assert_eq!(f.runtime.companion_snapshot(f.targets[0]).unwrap(), before);
    assert!(f.runtime.actor_spell_reserved(f.caster));
    assert!(f.runtime.actor_spell_reserved(f.targets[0]));
    assert_eq!(
        f.runtime
            .compare_assign_companion(f.caster, f.session, &before),
        Err(CarrierError::PlanConflict)
    );
    let competing = batch(&f, 2, vec![damage(f.targets[0], 0, 1)]);
    assert!(matches!(
        f.runtime.stage_spell_batch(&competing),
        Err(Error::SnapshotChanged)
    ));
    // A definite rollback frees the same genuine slots without changing their master.
    f.runtime
        .release_definitely_uncommitted_spell_batch(&staged)
        .unwrap();
    assert!(!f.runtime.actor_spell_reserved(f.caster));
    assert!(!f.runtime.actor_spell_reserved(f.targets[0]));
    assert_eq!(f.runtime.companion_snapshot(f.targets[0]).unwrap(), before);
    let mut retry = f.runtime.stage_spell_batch(&original).unwrap();
    f.runtime.reserve_spell_batch(&mut retry).unwrap();
    drop(retry);
    let retry = f.runtime.stage_spell_batch(&original).unwrap();
    assert!(f.runtime.commit_spell_batch(retry).unwrap().applied);
    let after = f.runtime.companion_snapshot(f.targets[0]).unwrap();
    assert_eq!(
        after.state.master,
        Some(super::super::runtime_actor_companion::CompanionMaster {
            actor: f.caster,
            session: f.session
        })
    );
    assert_eq!(
        (after.health, after.position, after.position_revision),
        (before.health, before.position, before.position_revision)
    );
    let replay = f.runtime.stage_spell_batch(&original).unwrap();
    assert!(!f.runtime.commit_spell_batch(replay).unwrap().applied);
    assert_eq!(f.runtime.companion_snapshot(f.targets[0]).unwrap(), after);
}

#[test]
fn master_assignment_refuses_substituted_target_session_and_stale_actual_predecessor() {
    let mut f = master_assignment_fixture();
    let original = master_assignment_batch(&f, 1);
    let mut substituted = original.clone();
    substituted.effects[0].target = f.targets[1];
    assert!(matches!(
        f.runtime.stage_spell_batch(&substituted),
        Err(Error::InvalidCondition)
    ));
    let mut foreign_session = original.clone();
    foreign_session.command = CommandRef::new(
        GameSessionId::decode(&uuid(0x25)).unwrap(),
        original.command.command_id(),
    );
    assert!(f.runtime.stage_spell_batch(&foreign_session).is_err());
    assert_eq!(
        f.runtime
            .companion_snapshot(f.targets[0])
            .unwrap()
            .state
            .master,
        None
    );
    let staged = f.runtime.stage_spell_batch(&original).unwrap();
    let before = f.runtime.companion_snapshot(f.targets[0]).unwrap();
    // Change only the genuine current master after staging, through its actual owner.
    f.runtime
        .compare_assign_companion(f.caster, f.session, &before)
        .unwrap();
    assert!(matches!(
        f.runtime.commit_spell_batch(staged),
        Err(Error::SnapshotChanged)
    ));
    assert!(f.runtime.stage_spell_batch(&original).is_err());
    assert_eq!(health(&f, 0), 20);
    assert_eq!(
        f.runtime
            .companion_snapshot(f.targets[1])
            .unwrap()
            .state
            .master,
        None
    );
}

#[test]
fn master_assignment_payment_requires_real_player_preflight_and_cannot_be_deferred() {
    use super::super::runtime_actor_spell_types::DeferredCommitAuthority;
    let mut f = master_assignment_fixture();
    let original = master_assignment_batch(&f, 1);
    let mut paid = original.clone();
    paid.anchor = Some(SpellAnchor {
        expected_revision: 1,
        next_revision: 2,
        paid_mana: 200,
        paid_soul: 0,
        cooldown_deadlines: vec![],
    });
    let staged = f.runtime.stage_spell_batch(&paid).unwrap();
    assert!(matches!(
        f.runtime.commit_spell_batch(staged),
        Err(Error::PlayerVitalsOwnerRequired)
    ));
    assert_eq!(
        f.runtime
            .companion_snapshot(f.targets[0])
            .unwrap()
            .state
            .master,
        None
    );
    let mut due = original.clone();
    let scope = super::super::super::RuntimeScopeRefV1::channel(
        f.runtime.binding().world_id(),
        f.runtime.binding().channel_id(),
    );
    let stamp = f.runtime.issue_owner_work().unwrap();
    due.deferred = Some(
        DeferredCommitAuthority::from_due(
            f.runtime.owner_fence().unwrap(),
            scope,
            f.runtime.binding().scope_generation(),
            stamp,
            1,
            1,
            original.binding.clone(),
            &due,
        )
        .unwrap(),
    );
    assert!(matches!(
        f.runtime.stage_spell_batch(&due),
        Err(Error::InvalidCondition)
    ));
    assert_eq!(
        f.runtime
            .companion_snapshot(f.targets[0])
            .unwrap()
            .state
            .master,
        None
    );
}
