//! Live TARGET-MOVE wire tests share the real bundle/PG fixture with item autowalk.
use super::TestResult;
use super::autowalk::{at, place, read, send};
use super::fixture::{debug, id};
use crate::foundation::{ExactActorRef, GameSessionId, MovementFacing, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::{
    MessageType, decode_command_result, decode_state_delta, decode_wire_envelope,
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
    send(client, 2, 81, &[26, 2, 8, 1]).await?;
    result(client, 2, 1).await?;
    let current = owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?;
    assert_eq!(current.position(), at(6, 4));
    assert_eq!(current.facing(), Some(MovementFacing::North));
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 1 {
                let value = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_item_handles(delta.payload).map_err(debug)?;
                if let Some(entity) = value
                    .update
                    .iter()
                    .find(|e| e.entity.identity == actor.placement_identity())
                {
                    assert!(matches!(
                        entity.detail,
                        oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                            direction: oteryn_protocol_oteryn::world_spatial::StepDirection::North,
                            ..
                        }
                    ));
                    break;
                }
            }
        }
    }

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
    tokio::time::sleep(Duration::from_millis(350)).await;
    let lost = position(owner, actor).await?;
    tokio::time::sleep(Duration::from_millis(350)).await;
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
            .commit_spell_relocation(Relocation(
                rat,
                old,
                MovementLocalPosition {
                    floor: 8,
                    ..old.position()
                },
            ))
            .map_err(debug)?;
        old.position()
    };
    tokio::time::sleep(Duration::from_millis(350)).await;
    let floor_stopped = position(owner, actor).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let now = runtime
            .borrow_movement_position()
            .read(rat)
            .map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_spell_relocation(Relocation(rat, now, at(old.x, 1)))
            .map_err(debug)?;
    }
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(position(owner, actor).await?, floor_stopped);
    send(client, 12, 81, &rat_follow).await?;
    result(client, 12, 1).await?;
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
            occurrence: crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(
                99,
            ))
            .map_err(debug)?,
            cell: death_cell,
        },
    ));
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert_eq!(
        position(owner, actor).await?,
        death_cell,
        "death stops the next follow step"
    );
    send(client, 13, 81, &[26, 2, 8, 2]).await?;
    result(client, 13, 3).await?;
    Ok(())
}

struct Relocation(
    ExactActorRef,
    crate::foundation::MovementPositionSnapshot,
    MovementLocalPosition,
);
impl crate::foundation::relocation_seal::Sealed for Relocation {}
impl crate::foundation::SpellRelocationProof for Relocation {
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        crate::foundation::MovementPositionSnapshot,
        MovementLocalPosition,
    ) {
        (self.0, self.1, self.2)
    }
}
pub(super) async fn without_capability(client: &mut DuplexStream) -> TestResult {
    loop {
        let frame = read(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    send(client, 14, 81, &[26, 2, 8, 2]).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, 14);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Rejected
            );
            assert!(result.payload.is_empty());
            return Ok(());
        }
    }
}
