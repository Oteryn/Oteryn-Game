//! Live TARGET-MOVE wire tests share the real bundle/PG fixture with item autowalk.

use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::TestResult;

use super::autowalk_fixtures::{at, place, read, send};

use super::fixture::{debug, id};

use crate::foundation::{ExactActorRef, GameSessionId, MovementFacing, MovementLocalPosition};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::AdmittedSession;

use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_state_delta, decode_wire_envelope,
};

use oteryn_protocol_oteryn::target_move::{
    COMMAND_TYPE_TARGET_MOVE_INTENT, TargetMoveIntent, encode_target_move_intent,
};
use oteryn_protocol_oteryn::world_spatial::{ActorPosition, StepDirection};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, WorldSpatialEntity, decode_world_spatial_entities_delta_with_item_handles,
};

use std::time::Duration;

use tokio::io::DuplexStream;

fn cell(x: u8, y: u8) -> Vec<u8> {
    vec![10, 6, 8, x * 2, 16, y * 2, 24, 14]
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?
        .position())
}

async fn result(client: &mut DuplexStream, id: u64, expected: u8) -> TestResult {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            assert_eq!(result.payload, [8, expected]);
            return Ok(());
        }
    }
}

async fn follow_state(
    client: &mut DuplexStream,
    id: u64,
    expected: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
) -> TestResult {
    send(client, id, 6371, &[]).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            assert_eq!(result.status, CommandStatus::Accepted);
            assert_eq!(
                oteryn_protocol_oteryn::attack::decode_attack_target_intent(result.payload)
                    .map_err(debug)?,
                expected
            );
            return Ok(());
        }
    }
}

async fn follow_check_barrier(
    client: &mut DuplexStream,
    id: u64,
    kind: u32,
    payload: &[u8],
    accepted_payload: &[u8],
) -> TestResult {
    // Replays observe the connection loop without replacing FOLLOW. The first ACK
    // finishes any earlier owner work. After the wait, a due FOLLOW check must finish
    // before the second remaining ACK, even when one ready frame wins the first turn.
    for replay in 0..3 {
        if replay == 1 {
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
        send(client, id, kind, payload).await?;
        loop {
            let frame = read(client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                assert_eq!(result.command_id, id);
                assert_eq!(result.status, CommandStatus::DuplicateReplay);
                assert_eq!(result.payload, accepted_payload);
                break;
            }
        }
    }
    Ok(())
}

async fn next_step(client: &mut DuplexStream) -> TestResult<MovementLocalPosition> {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 1 {
                let value = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_item_handles(delta.payload).map_err(debug)?;
                let at = value.actor_position;
                return Ok(MovementLocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                });
            }
        }
    }
}

pub(super) async fn client_scenario(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    loop {
        let frame = read(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    // Exact destination, not its adjacent tile, and a wall requires a cardinal detour.
    send(client, 1, 81, &cell(6, 4)).await?;
    result(client, 1, 1).await?;
    let mut previous = at(1, 1);
    let started = tokio::time::Instant::now();
    let mut steps = 0;
    loop {
        let current = next_step(client).await?;
        assert_eq!(current.floor, previous.floor);
        assert_eq!(
            current.x.abs_diff(previous.x) + current.y.abs_diff(previous.y),
            1,
            "never teleport"
        );
        assert!(!(current.x == 3 && current.y <= 2), "wall collision");
        previous = current;
        steps += 1;
        if current == at(6, 4) {
            break;
        }
    }
    assert!(steps >= 8);
    assert!(
        started.elapsed() >= Duration::from_millis(50 * (steps - 1)),
        "paced by Movement"
    );
    // TURN changes the actual owner facing used by directional spells, without relocation.
    turn(
        client,
        2,
        actor,
        at(6, 4),
        oteryn_protocol_oteryn::world_spatial::StepDirection::North,
    )
    .await?;
    let current = owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?;
    assert_eq!(current.position(), at(6, 4));
    assert_eq!(current.facing(), Some(MovementFacing::North));

    // Stop discards the queue before another due step.
    send(client, 3, 81, &cell(1, 4)).await?;
    result(client, 3, 1).await?;
    let stopped = next_step(client).await?;
    send(client, 4, 81, &[16, 1]).await?;
    result(client, 4, 1).await?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(position(owner, actor).await?, stopped);
    // A real visible player is resolved by identity AND generation, on this Channel.
    let target_session = GameSessionId::decode(&id(77)).map_err(debug)?;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(target_session)
            .map_err(debug)?;
        let target = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(target, at(7, 5))
            .map_err(debug)?;
        target
    };
    let reference = {
        let runtime = owner.runtime.lock().await;
        let census = runtime.visible_entities();
        let player = census
            .players
            .iter()
            .find(|p| p.actor == target)
            .ok_or("target census")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation: player.generation,
        }
    };
    let encoded = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&reference))
        .map_err(debug)?;
    let mut follow = vec![34, u8::try_from(encoded.len())?];
    follow.extend_from_slice(&encoded);
    send(client, 5, 81, &follow).await?;
    result(client, 5, 1).await?;
    loop {
        let current = next_step(client).await?;
        if crate::movement::path_search::chebyshev(current, at(7, 5)) == 1 {
            break;
        }
    }
    // Move the target with the actual cardinal owner and require a fresh follow route.
    let target_admitted = AdmittedSession {
        runtime_actor: Some(target),
        game_session_id: target_session,
        ..admitted
    };
    place(owner, target_admitted, at(7, 1)).await?;
    loop {
        let current = next_step(client).await?;
        if current.floor == 7 && crate::movement::path_search::chebyshev(current, at(7, 1)) == 1 {
            break;
        }
    }
    // Logout removes the exact target. It cannot be silently substituted by another actor.
    owner
        .runtime
        .lock()
        .await
        .remove_terminal_session(target_session, target)
        .map_err(debug)?;
    follow_check_barrier(client, 5, 81, &follow, &[8, 1]).await?;
    let lost = position(owner, actor).await?;
    follow_check_barrier(client, 5, 81, &follow, &[8, 1]).await?;
    assert_eq!(position(owner, actor).await?, lost);
    send(client, 6, 81, &follow).await?;
    result(client, 6, 3).await?;
    // No route across the second wall; malformed oneof cannot turn or move.
    send(client, 7, 81, &cell(12, 1)).await?;
    result(client, 7, 2).await?;
    send(client, 8, 81, &[16, 1, 16, 1]).await?;
    result(client, 8, 3).await?;
    assert_eq!(position(owner, actor).await?, lost);
    // A new single-step command cancels a queued destination.
    send(client, 9, 81, &cell(4, 5)).await?;
    result(client, 9, 1).await?;
    next_step(client).await?;
    send(client, 10, 1, &[8, 4]).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            assert_eq!(decode_command_result(envelope.payload())?.command_id, 10);
            break;
        }
    }
    tokio::time::sleep(Duration::from_millis(350)).await;
    let manual = position(owner, actor).await?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(position(owner, actor).await?, manual);
    // FOLLOW also resolves a living creature through the same production command.
    let rat = {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn(pin, "oteryn:creature.rat", at(7, 5))
            .map_err(debug)?
            .actor
    };
    let rat_ref = {
        let runtime = owner.runtime.lock().await;
        let census = runtime.visible_entities();
        let rat = census
            .creatures
            .iter()
            .find(|c| c.actor == rat)
            .ok_or("rat")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: rat.actor.placement_identity(),
            generation: rat.generation,
        }
    };
    let encoded = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&rat_ref))
        .map_err(debug)?;
    let mut rat_follow = vec![34, u8::try_from(encoded.len())?];
    rat_follow.extend_from_slice(&encoded);
    send(client, 11, 81, &rat_follow).await?;
    result(client, 11, 1).await?;
    follow_state(client, 12, Some(rat_ref)).await?;
    loop {
        let current = next_step(client).await?;
        if crate::movement::path_search::chebyshev(current, at(7, 5)) == 1 {
            break;
        }
    }
    // An independent floor relocation ends follow. Returning the target cannot resume it.
    let old = {
        let mut runtime = owner.runtime.lock().await;
        let old = runtime
            .borrow_movement_position()
            .read(rat)
            .map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                rat,
                old,
                MovementLocalPosition {
                    floor: 8,
                    ..old.position()
                },
            )
            .map_err(debug)?;
        old.position()
    };
    follow_check_barrier(client, 12, 6371, &[], &encoded).await?;
    follow_state(client, 13, None).await?;
    let floor_stopped = position(owner, actor).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let now = runtime
            .borrow_movement_position()
            .read(rat)
            .map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(rat, now, at(old.x, 1))
            .map_err(debug)?;
    }
    follow_check_barrier(client, 13, 6371, &[], &[]).await?;
    assert_eq!(position(owner, actor).await?, floor_stopped);
    send(client, 14, 81, &rat_follow).await?;
    result(client, 14, 1).await?;
    loop {
        let current = next_step(client).await?;
        if current != floor_stopped {
            break;
        }
    }
    let death_cell = position(owner, actor).await?;
    owner.states.spell_states.lock().await.deaths.push((
        actor,
        admitted.game_session_id,
        crate::gameplay_transport::actor_spell::PlayerDeath {
            public_cause: None,
            occurrence: crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(
                99,
            ))
            .map_err(debug)?,
            cell: death_cell,
        },
    ));
    follow_check_barrier(client, 14, 81, &rat_follow, &[8, 1]).await?;
    assert_eq!(
        position(owner, actor).await?,
        death_cell,
        "death stops the next follow step"
    );
    send(client, 15, 81, &[26, 2, 8, 2]).await?;
    result(client, 15, 3).await?;
    Ok(())
}

pub(super) async fn without_capability(client: &mut DuplexStream, corpse: [u8; 16]) -> TestResult {
    let mut saw_corpse = false;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?.1 {
                assert_ne!(domain.domain_id, 17, "map view was not selected");
                if domain.domain_id == 1 {
                    let snapshot = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles(domain.payload).map_err(debug)?;
                    assert_eq!(snapshot.actor_position.floor, 7);
                    let entry = snapshot.entities.iter().find(|entry| entry.entity.identity == corpse).ok_or("corpse without map view")?;
                    assert_eq!(entry.position.floor, snapshot.actor_position.floor);
                    saw_corpse = true;
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    assert!(saw_corpse);
    for (id, kind, payload) in [(16, 81, &[26, 2, 8, 2][..]), (17, 6371, &[][..])] {
        send(client, id, kind, payload).await?;
        loop {
            let frame = read(client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                assert_eq!(result.command_id, id);
                assert_eq!(result.status, CommandStatus::Rejected);
                assert!(result.payload.is_empty());
                break;
            }
        }
    }
    Ok(())
}

pub(super) async fn turn(
    client: &mut DuplexStream,
    command: u64,
    caster: ExactActorRef,
    cell: MovementLocalPosition,
    facing: StepDirection,
) -> TestResult {
    let intent = encode_target_move_intent(TargetMoveIntent::Turn(facing)).map_err(debug)?;
    send(client, command, COMMAND_TYPE_TARGET_MOVE_INTENT, &intent).await?;
    let mut saw_turn = false;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, command);
            assert_eq!(result.status, CommandStatus::Accepted);
            assert_eq!(result.payload, [8, 1]);
            assert!(
                saw_turn,
                "TURN {command} receipt preceded the facing update"
            );
            return Ok(());
        }
        saw_turn |= update_in(&envelope, caster, cell, facing)?;
    }
}

pub(super) fn update_in(
    envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    caster: ExactActorRef,
    cell: MovementLocalPosition,
    facing: StepDirection,
) -> TestResult<bool> {
    if envelope.message_type() == MessageType::StateDelta {
        let delta = decode_state_delta(envelope.payload())?;
        if delta.domain_id == 1 {
            let spatial = decode_world_spatial_entities_delta_with_item_handles(delta.payload)
                .map_err(debug)?;
            return Ok(sees(&spatial.update, caster, cell, facing));
        }
    }
    Ok(false)
}

pub(super) fn sees(
    entities: &[WorldSpatialEntity],
    caster: ExactActorRef,
    cell: MovementLocalPosition,
    facing: StepDirection,
) -> bool {
    let position = ActorPosition {
        x: cell.x,
        y: cell.y,
        floor: cell.floor,
    };
    entities.iter().any(|entry| {
        entry.entity.identity == caster.placement_identity()
            && entry.entity.generation == caster.actor_local_generation()
            && entry.position == position
            && matches!(entry.detail,
                EntityDetail::Actor { direction, .. }
                | EntityDetail::ActorWithIcons { direction, .. } if direction == facing)
    })
}
