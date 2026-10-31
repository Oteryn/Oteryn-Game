use super::*;
use crate::foundation::{ChannelId, CharacterId, NodeId, RuntimeScopeRefV1, WorldId};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

struct Fixture {
    source: ChannelRuntimeV1,
    destination: ChannelRuntimeV1,
    member: ActivityTransferMember,
    target: ExactActorRef,
}

fn fixture() -> Result<Fixture, String> {
    let world = WorldId::decode(&id(1)).map_err(debug)?;
    let node = NodeId::decode(&id(2)).map_err(debug)?;
    let source_scope = RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(3)).map_err(debug)?);
    let destination_scope = RuntimeScopeRefV1::instance(world, id(4)).map_err(debug)?;
    let runtime = |scope| {
        ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::from_activation(
                world,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (2, 11, -7),
            ),
        )
        .map_err(debug)
    };
    let mut source = runtime(source_scope)?;
    let destination = runtime(destination_scope)?;
    let source_session = GameSessionId::decode(&id(5)).map_err(debug)?;
    let destination_session = GameSessionId::decode(&id(6)).map_err(debug)?;
    let reservation = source
        .reserve_fresh_session(source_session)
        .map_err(debug)?;
    let source_actor = source.commit_fresh_session(reservation).map_err(debug)?;
    source
        .initialize_first_entry_position(source_actor)
        .map_err(debug)?;
    let character = CharacterId::decode(&id(7)).map_err(debug)?;
    let source_lease = CharacterLease::new(character, 1).map_err(debug)?;
    source
        .bind_attacker_lease(source_actor, source_session, source_lease)
        .map_err(debug)?;
    let target_session = GameSessionId::decode(&id(8)).map_err(debug)?;
    let target = source
        .reserve_fresh_session(target_session)
        .map_err(debug)?;
    let target = source.commit_fresh_session(target).map_err(debug)?;
    source
        .initialize_first_entry_position(target)
        .map_err(debug)?;
    let member = ActivityTransferMember {
        source_actor,
        source_session,
        source_lease,
        destination_session,
        destination_lease: CharacterLease::new(character, 2).map_err(debug)?,
        destination_cell: MovementLocalPosition {
            x: 2,
            y: 11,
            floor: -7,
        },
    };
    Ok(Fixture {
        source,
        destination,
        member,
        target,
    })
}

fn install_live_attack(
    states: &mut ChannelAttackStates,
    fixture: &Fixture,
    now: SemanticTimeMicros,
) -> Result<(), String> {
    let member = fixture.member;
    assert!(states.set_expert_modes(
        &fixture.source,
        member.source_actor,
        member.source_session,
        ExpertFightModes {
            modes: FightModes {
                fight_mode: WireFightMode::Defensive,
                chase: ChaseMode::Chase,
                secure: false,
            },
            expert: ExpertMode::RedFist,
        },
    ));
    states.record_hit_taken(
        &fixture.source,
        member.source_actor,
        member.source_session,
        now,
    );
    assert!(
        states
            .take_player_block(
                &fixture.source,
                member.source_actor,
                member.source_session,
                now,
            )
            .is_some()
    );
    let index = states
        .index(member.source_actor, member.source_session)
        .ok_or("attack entry")?;
    let entry = &mut states.entries[index];
    let facts = TargetFacts {
        present_and_visible: true,
        attackable_kind: true,
        alive: true,
        same_floor: true,
        distance: 1,
        attacker_in_protection_zone: false,
        target_in_protection_zone: false,
        attacker_reentry_protected: false,
        target_reentry_protected: false,
    };
    entry
        .state
        .set_target(
            fixture.target,
            CommandRef::new(member.source_session, CommandId::new(1).map_err(debug)?),
            &facts,
        )
        .map_err(debug)?;
    let constants = AttackConstants::checked_in().ok_or("checked-in attack constants")?;
    let _swing = entry
        .state
        .poll_swing(now, constants.attack_interval_micros(), &facts);
    entry.target_ref = Some(EntityRef {
        identity: fixture.target.placement_identity(),
        generation: 1,
    });
    entry.target_set_at = now;
    assert!(entry.record_shared_experience_activity(&fixture.source, now));
    Ok(())
}

#[test]
fn activity_attack_transfer_preserves_deadline_modes_budget_and_pacing_after_source_prune()
-> Result<(), String> {
    let mut f = fixture()?;
    let mut source = ChannelAttackStates::default();
    let mut destination = ChannelAttackStates::default();
    let now = SemanticTimeMicros::from_micros(10_000);
    install_live_attack(&mut source, &f, now)?;
    let index = source
        .index(f.member.source_actor, f.member.source_session)
        .ok_or("source entry")?;
    let mut expected = source.entries[index].state.clone();
    expected.clear_target();
    let modes = source.expert_modes(f.member.source_actor, f.member.source_session);
    let mut budget = source.entries[index]
        .defence_blocks
        .clone()
        .ok_or("spent defence budget")?;
    let clock_now = Instant::now();
    let due = clock_now + Duration::from_millis(250);
    let step = Arc::new(());
    assert!(
        source
            .begin_step(f.member.source_actor, &step, clock_now)
            .is_none()
    );
    source.record_step_deadline(f.member.source_actor, &step, clock_now, Some(due));
    let mut held = source
        .prepare_activity_attacks(&f.source, &mut destination, &[f.member])
        .map_err(|_| "attack extraction")?;
    let mut physical = f
        .source
        .prepare_activity_group_entry(&mut f.destination, &[f.member])
        .map_err(debug)?;
    source.prune(&f.source);
    assert!(
        source
            .index(f.member.source_actor, f.member.source_session)
            .is_none()
    );
    let moved = f
        .source
        .commit_activity_transfer(&mut f.destination, &mut physical)
        .map_err(debug)?;
    let next = *moved.first().ok_or("moved actor")?;
    assert!(matches!(
        next.destination_actor.runtime_scope(),
        RuntimeScopeRefV1::Instance { .. }
    ));
    let mut conflicting = next;
    conflicting.destination_session = GameSessionId::decode(&id(99)).map_err(debug)?;
    assert!(
        destination
            .prepare_activity_attack_commit(&mut held, &[conflicting])
            .is_err()
    );
    assert!(destination.entries.is_empty());
    destination
        .prepare_activity_attack_commit(&mut held, &moved)
        .map_err(|_| "attack install preflight")?
        .commit();
    let index = destination
        .index(next.destination_actor, next.destination_session)
        .ok_or("destination entry")?;
    assert_eq!(destination.entries[index].state, expected);
    assert_eq!(
        destination.expert_modes(next.destination_actor, next.destination_session),
        modes
    );
    assert_eq!(
        destination
            .combat_state(next.destination_actor, next.destination_session, now)
            .target,
        None
    );
    let deadline = expected.in_fight_until().ok_or("original PvE deadline")?;
    assert_eq!(
        destination.in_fight_until(next.destination_actor, next.destination_session, now),
        Some(deadline)
    );
    assert_eq!(
        destination.in_fight_until(next.destination_actor, next.destination_session, deadline),
        None
    );
    assert_eq!(
        destination.step_deadline(next.destination_actor, clock_now),
        Some(due)
    );
    let activity = destination.entries[index]
        .shared_experience_activity
        .ok_or("shared activity")?;
    assert_eq!(
        activity.character,
        *f.member.source_lease.character_id().as_bytes()
    );
    assert_eq!(
        activity.lease_generation,
        next.destination_lease.generation()
    );
    assert_eq!(
        activity.at, now,
        "transfer never renews the source observation"
    );
    let constants = AttackConstants::checked_in().ok_or("checked-in attack constants")?;
    while budget.try_block(&constants.block, now) {
        assert!(
            destination
                .take_player_block(
                    &f.destination,
                    next.destination_actor,
                    next.destination_session,
                    now
                )
                .is_some()
        );
    }
    assert!(
        destination
            .take_player_block(
                &f.destination,
                next.destination_actor,
                next.destination_session,
                now
            )
            .is_none()
    );
    assert_eq!(
        source.in_fight_until(f.member.source_actor, f.member.source_session, now),
        None
    );
    Ok(())
}

#[test]
fn activity_attack_refusal_restores_exact_target_and_deadline_and_busy_step_extracts_nothing()
-> Result<(), String> {
    let mut f = fixture()?;
    let mut source = ChannelAttackStates::default();
    let mut destination = ChannelAttackStates::default();
    let now = SemanticTimeMicros::from_micros(10_000);
    install_live_attack(&mut source, &f, now)?;
    let index = source
        .index(f.member.source_actor, f.member.source_session)
        .ok_or("source entry")?;
    let expected = source.entries[index].state.clone();
    let modes = source.expert_modes(f.member.source_actor, f.member.source_session);
    let step = Arc::new(());
    let clock_now = Instant::now();
    assert!(
        source
            .begin_step(f.member.source_actor, &step, clock_now)
            .is_none()
    );
    assert!(
        source
            .prepare_activity_attacks(&f.source, &mut destination, &[f.member])
            .is_err()
    );
    assert_eq!(source.entries[index].state, expected);
    source.record_step_deadline(f.member.source_actor, &step, clock_now, None);
    let mut held = source
        .prepare_activity_attacks(&f.source, &mut destination, &[f.member])
        .map_err(|_| "attack extraction")?;
    let mut physical = f
        .source
        .prepare_activity_group_entry(&mut f.destination, &[f.member])
        .map_err(debug)?;
    source.prune(&f.source);
    f.source
        .abort_activity_transfer(&mut f.destination, &mut physical)
        .map_err(debug)?;
    source.abort_activity_attacks(&mut held);
    let index = source
        .index(f.member.source_actor, f.member.source_session)
        .ok_or("restored entry")?;
    assert_eq!(source.entries[index].state, expected);
    assert_eq!(
        source.entries[index]
            .target_ref
            .map(|target| target.identity),
        Some(f.target.placement_identity())
    );
    assert_eq!(
        source.expert_modes(f.member.source_actor, f.member.source_session),
        modes
    );
    assert!(destination.entries.is_empty());
    Ok(())
}
