//! ACTIVITY_SCOPE is a fresh session barrier, not an opaque ordinary replacement.
use super::*;
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
};

fn switch(source: u8, destination: u8, generation: u64, channel: bool) -> ActivityScopeSnapshot {
    ActivityScopeSnapshot {
        handoff: uuid_v7(destination + 20),
        source_session: uuid_v7(source),
        session: uuid_v7(destination),
        scope: if channel {
            ActivityScope::Channel {
                world: uuid_v7(2),
                channel: uuid_v7(3),
            }
        } else {
            ActivityScope::Instance {
                world: uuid_v7(2),
                instance: uuid_v7(11),
            }
        },
        scope_generation: generation,
        connection_generation: generation,
        lease_generation: if channel { 4 } else { 3 },
        selection: vec![117],
        requires_ack: false,
    }
}

async fn publish_switch(
    peer: &mut DuplexStream,
    source_generation: u64,
    sequence: u64,
    scope: &ActivityScopeSnapshot,
) -> Result<(), BoxError> {
    let payload = encode_activity_scope(scope).map_err(|error| format!("{error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        source_generation,
        99,
        sequence,
        &[DomainSnapshot {
            domain_id: 99,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        }],
    )? {
        write_frame(peer, &frame).await?;
    }
    Ok(())
}

async fn destination(peer: &mut DuplexStream, generation: u64, x: i32) -> Result<(), BoxError> {
    let scope = if x == 200 {
        switch(1, 10, generation, false)
    } else {
        switch(10, 12, generation, true)
    };
    destination_projection(peer, generation, x, scope).await
}

async fn destination_projection(
    peer: &mut DuplexStream,
    generation: u64,
    x: i32,
    scope: ActivityScopeSnapshot,
) -> Result<(), BoxError> {
    let spatial = encode_world_spatial(&WireSpatialObservation {
        content_generation: [x as u8; 32],
        actor_position: ActorPosition {
            x,
            y: 31151,
            floor: -8,
        },
    });
    let overlay = encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
    // Real serve/setup emits the same retained binding before destination spatial state.
    let scope = encode_activity_scope(&scope).map_err(|e| format!("{e:?}"))?;
    for frame in encode_single_chunk_snapshot(
        generation,
        100,
        1,
        &[
            DomainSnapshot {
                domain_id: 99,
                revision: generation,
                snapshot_type: 1,
                payload: &scope,
            },
            DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 2,
                revision: 1,
                snapshot_type: 1,
                payload: &overlay,
            },
        ],
    )? {
        write_frame(peer, &frame).await?;
    }
    Ok(())
}

#[test]
fn srx3_activity_scope_instance_return_resets_session_sequence_and_command_stream()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut peer) = tokio::io::duplex(256 * 1024);
                let server = tokio::spawn(async move {
                    join_peer_with(&mut peer, &[117], &[117], &[], (1, spatial(100)), &[]).await?;
                    publish_switch(&mut peer, 1, 41, &switch(1, 10, 1, false)).await?;
                    destination(&mut peer, 1, 200).await?;
                    for (generation, current, next) in [(1, 10, Some(switch(10, 12, 1, true))), (1, 12, None)] {
                        let frame = read_frame(&mut peer).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        assert_eq!(envelope.connection_generation(), generation);
                        assert_eq!(envelope.client_command(generation)?.command_id, 1);
                        assert_eq!(envelope.client_command_session_id()?, Some(uuid_v7(current).as_slice()));
                        write_frame(
                            &mut peer,
                            &encode_command_result(
                                generation,
                                2,
                                1,
                                CommandStatus::Accepted,
                                &encode_step_result(StepDisposition::Blocked),
                            )?,
                        )
                        .await?;
                        if let Some(scope) = next {
                            publish_switch(&mut peer, generation, 3, &scope).await?;
                            destination(&mut peer, 1, 100).await?;
                        }
                    }
                    let mut rest = Vec::new();
                    let _ = peer.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[117])?).await?;
                for (destination, position) in [(10, 200), (12, 100)] {
                    let events = session.wait_for_events(DEADLINE).await?;
                    assert!(matches!(events.first(), Some(SessionEvent::ActivityScope(delta)) if delta.value.session == uuid_v7(destination)));
                    assert_eq!(
                        session.join_snapshot().game_session_id,
                        GameSessionId::decode(&uuid_v7(destination))?
                    );
                    assert_eq!(
                        session.join_snapshot().world_spatial.actor_position.x,
                        position
                    );
                    assert_eq!(session.next_command_id(), 1);
                    assert_eq!(
                        session.step(StepDirection::North).await?.disposition,
                        StepDisposition::Blocked
                    );
                }
                drop(session);
                server.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity scope test thread panicked")?
}

#[test]
fn srx3_activity_scope_rejects_wrong_source_and_widened_selection_before_replacement()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for invalid in 0..3 {
                    let (client, mut peer) = tokio::io::duplex(256 * 1024);
                    let server = tokio::spawn(async move {
                        join_peer_with(&mut peer, &[117], &[117], &[], (1, spatial(100)), &[])
                            .await?;
                        let mut value = switch(if invalid == 1 { 90 } else { 1 }, 10, 7, false);
                        match invalid {
                            0 => value.selection = vec![4, 117],
                            2 => {
                                value.scope = ActivityScope::Instance {
                                    world: uuid_v7(99),
                                    instance: uuid_v7(11),
                                }
                            }
                            _ => {}
                        }
                        publish_switch(&mut peer, 1, 41, &value).await?;
                        let mut rest = Vec::new();
                        let _ = peer.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[117])?).await?;
                    assert!(session.wait_for_events(DEADLINE).await.is_err());
                    assert_eq!(
                        session.join_snapshot().game_session_id,
                        GameSessionId::decode(&uuid_v7(1))?
                    );
                    assert_eq!(session.next_command_id(), 7);
                    drop(session);
                    server.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity scope negative test thread panicked")?
}

#[test]
fn srx3_activity_scope_cancels_inflight_old_command_without_consuming_fresh_command_one()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut peer) = tokio::io::duplex(256 * 1024);
                let server = tokio::spawn(async move {
                    read_frame(&mut peer).await?;
                    write_frame(
                        &mut peer,
                        &encode_server_accepted(&ServerAcceptedValue {
                            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                            world_id: WorldId::decode(&uuid_v7(2))?,
                            channel_id: ChannelId::decode(&uuid_v7(3))?,
                            connection_generation: 1,
                            current_server_sequence: 0,
                            next_command_id: 1,
                            schema_revision: 1,
                            selected_capabilities: &[117],
                        })?,
                    )
                    .await?;
                    let spatial = spatial(100);
                    let overlay =
                        encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
                    for frame in encode_single_chunk_snapshot(
                        1,
                        1,
                        40,
                        &[
                            DomainSnapshot {
                                domain_id: 1,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &spatial,
                            },
                            DomainSnapshot {
                                domain_id: 2,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &overlay,
                            },
                        ],
                    )? {
                        write_frame(&mut peer, &frame).await?;
                    }
                    // Publish a non-initiator's handoff before consuming its already queued source frame.
                    publish_switch(&mut peer, 1, 41, &switch(1, 10, 1, false)).await?;
                    destination(&mut peer, 1, 200).await?;
                    let old = read_frame(&mut peer).await?;
                    assert_eq!(decode_wire_envelope(&old)?.client_command(1)?.command_id, 1);
                    // There is deliberately no fabricated outcome for this terminal source request.
                    let fresh = read_frame(&mut peer).await?;
                    assert_eq!(
                        decode_wire_envelope(&fresh)?.client_command_session_id()?,
                        Some(uuid_v7(10).as_slice())
                    );
                    assert_eq!(
                        decode_wire_envelope(&fresh)?.client_command(1)?.command_id,
                        1
                    );
                    write_frame(
                        &mut peer,
                        &encode_command_result(
                            1,
                            2,
                            1,
                            CommandStatus::Accepted,
                            &encode_step_result(StepDisposition::Blocked),
                        )?,
                    )
                    .await?;
                    let mut rest = Vec::new();
                    let _ = peer.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[117])?).await?;
                assert!(matches!(
                    session.step(StepDirection::North).await,
                    Err(SessionError::ScopeChanged)
                ));
                assert_eq!(
                    session.join_snapshot().game_session_id,
                    GameSessionId::decode(&uuid_v7(10))?
                );
                assert_eq!(session.next_command_id(), 1);
                assert_eq!(
                    session.step(StepDirection::East).await?.disposition,
                    StepDisposition::Blocked
                );
                drop(session);
                server.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity scope cancellation thread panicked")?
}

#[path = "activity_scope_initial.rs"]
mod initial;

#[path = "activity_scope_recovery.rs"]
mod recovery;
