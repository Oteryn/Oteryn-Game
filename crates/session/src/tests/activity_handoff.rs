use super::*;
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
};

fn receipt(source: u8, destination: u8, generation: u64, returning: bool) -> ActivityScopeSnapshot {
    ActivityScopeSnapshot {
        handoff: uuid_v7(9),
        source_session: uuid_v7(source),
        session: uuid_v7(destination),
        scope: if returning {
            ActivityScope::Channel {
                world: uuid_v7(2),
                channel: uuid_v7(3),
            }
        } else {
            ActivityScope::Instance {
                world: uuid_v7(2),
                instance: uuid_v7(8),
            }
        },
        scope_generation: 1,
        connection_generation: generation,
        lease_generation: generation,
        selection: vec![117],
        requires_ack: false,
    }
}

async fn transfer(
    server: &mut DuplexStream,
    source_generation: u64,
    sequence: u64,
    receipt: &ActivityScopeSnapshot,
    destination: &ActivityScopeSnapshot,
    destination_sequence: u64,
    include_spatial: bool,
) -> Result<(), BoxError> {
    let payload = encode_activity_scope(receipt).map_err(|e| format!("{e:?}"))?;
    for frame in encode_single_chunk_snapshot(
        source_generation,
        2,
        sequence,
        &[DomainSnapshot {
            domain_id: 99,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        }],
    )? {
        write_frame(server, &frame).await?;
    }
    let payload = encode_activity_scope(destination).map_err(|e| format!("{e:?}"))?;
    let position = spatial(if matches!(receipt.scope, ActivityScope::Instance { .. }) {
        123
    } else {
        0
    });
    let overlay = encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
    let map_selected = receipt.selection.contains(&18);
    let map = if map_selected {
        activity_map(
            if matches!(receipt.scope, ActivityScope::Instance { .. }) {
                123
            } else {
                0
            },
            if matches!(receipt.scope, ActivityScope::Instance { .. }) {
                88
            } else {
                99
            },
        )?
    } else {
        Vec::new()
    };
    let mut domains = vec![
        DomainSnapshot {
            domain_id: 99,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        },
        DomainSnapshot {
            domain_id: if map_selected { 17 } else { 2 },
            revision: 1,
            snapshot_type: 1,
            payload: if map_selected { &map } else { &overlay },
        },
    ];
    if include_spatial {
        domains.push(DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &position,
        });
    }
    for frame in encode_single_chunk_snapshot(
        receipt.connection_generation,
        1,
        destination_sequence,
        &domains,
    )? {
        write_frame(server, &frame).await?;
    }
    Ok(())
}

#[test]
fn soulpit_scope_barrier_adopts_fresh_instance_and_return_without_replaying_source_command()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for pending_command in [false, true] {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[117], &[117], &[], (1, spatial(0)), &[])
                            .await?;
                        if pending_command {
                            read_frame(&mut server).await?;
                        }
                        let entered = receipt(1, 4, 2, false);
                        transfer(&mut server, 1, 41, &entered, &entered, 0, true).await?;
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(2)?;
                        assert_eq!(command.command_id, 1, "source command is never retried");
                        assert_eq!(
                            envelope.client_command_session_id()?,
                            Some(uuid_v7(4).as_slice())
                        );
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                2,
                                1,
                                1,
                                CommandStatus::Accepted,
                                &encode_step_result(StepDisposition::Blocked),
                            )?,
                        )
                        .await?;
                        let returned = receipt(4, 5, 3, true);
                        transfer(&mut server, 2, 2, &returned, &returned, 0, true).await?;
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        assert_eq!(envelope.client_command(3)?.command_id, 1);
                        assert_eq!(
                            envelope.client_command_session_id()?,
                            Some(uuid_v7(5).as_slice())
                        );
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                3,
                                1,
                                1,
                                CommandStatus::Accepted,
                                &encode_step_result(StepDisposition::Blocked),
                            )?,
                        )
                        .await?;
                        let mut rest = vec![];
                        let _ = server.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[117])?).await?;
                    if pending_command {
                        let error = session
                            .step(StepDirection::North)
                            .await
                            .err()
                            .ok_or("source command must cancel")?;
                        assert_eq!(format!("{error:?}"), "ActivityScopeChanged");
                    } else {
                        session.wait_for_events(DEADLINE).await?;
                    }
                    assert_eq!(
                        session.join_snapshot().game_session_id.as_bytes(),
                        &uuid_v7(4)
                    );
                    assert_eq!(session.connection_generation, 2);
                    assert_eq!(session.activity_scope(), Some(&receipt(1, 4, 2, false)));
                    assert_eq!(session.world_spatial.actor_position.x, 123);
                    assert_eq!(session.last_server_sequence, 0);
                    assert!(!session.unusable);
                    session.take_events();
                    session.step(StepDirection::North).await?;
                    session.wait_for_events(DEADLINE).await?;
                    assert_eq!(
                        session.join_snapshot().game_session_id.as_bytes(),
                        &uuid_v7(5)
                    );
                    assert_eq!(session.world_spatial.actor_position.x, 0);
                    assert_eq!(session.connection_generation, 3);
                    assert_eq!(session.activity_scope(), Some(&receipt(4, 5, 3, true)));
                    session.step(StepDirection::North).await?;
                    drop(session);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity transfer thread failed")?
}

#[test]
fn soulpit_scope_barrier_rejects_wrong_source_selection_or_incomplete_destination_atomically()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..5 {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let (sent, ready) = tokio::sync::oneshot::channel();
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[117], &[117], &[], (1, spatial(0)), &[])
                            .await?;
                        let mut entered = receipt(1, 4, 2, false);
                        if fault == 0 {
                            entered.source_session = uuid_v7(99);
                        }
                        if fault == 1 {
                            entered.selection.push(168);
                        }
                        let mut destination = entered.clone();
                        if fault == 2 {
                            destination.scope_generation = 2;
                        }
                        transfer(
                            &mut server,
                            1,
                            41,
                            &entered,
                            &destination,
                            u64::from(fault == 3),
                            fault != 4,
                        )
                        .await?;
                        let _ = sent.send(());
                        let mut rest = vec![];
                        let _ = server.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[117])?).await?;
                    ready.await?;
                    assert!(
                        session
                            .service_liveness(Duration::from_millis(50))
                            .await
                            .is_err(),
                        "fault {fault}"
                    );
                    assert_eq!(
                        session.join_snapshot().game_session_id.as_bytes(),
                        &uuid_v7(1)
                    );
                    assert_eq!(session.world_spatial.actor_position.x, 0);
                    assert_eq!(session.connection_generation, 1);
                    assert!(session.unusable);
                    drop(session);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "invalid activity transfer thread failed")?
}

#[test]
fn soulpit_recovery_projection_is_bound_to_admitted_session_and_selection() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..7 {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(async move {
                        let mut restored = receipt(0, 1, 1, false);
                        if fault == 1 {
                            restored.session = uuid_v7(4);
                        }
                        if fault == 2 {
                            restored.connection_generation = 2;
                        }
                        if fault == 3 {
                            restored.selection.push(168);
                        }
                        let payload = if fault == 4 {
                            vec![255]
                        } else {
                            encode_activity_scope(&restored).map_err(|e| format!("{e:?}"))?
                        };
                        let mut projections =
                            vec![(99, 1, if fault == 5 { 2 } else { 1 }, payload)];
                        if fault == 6 {
                            // Even a valid gameplay domain may not precede
                            // the retained binding after the two core domains.
                            projections.insert(
                                0,
                                (
                                    3,
                                    1,
                                    1,
                                    actor_spell::encode_actor_vitals(&vitals(150, 55))
                                        .map_err(|error| format!("{error:?}"))?,
                                ),
                            );
                        }
                        join_peer_with(
                            &mut server,
                            &[117],
                            &[117],
                            &[],
                            (1, spatial(0)),
                            &projections,
                        )
                        .await?;
                        Ok::<(), BoxError>(())
                    });
                    let result = Session::admit(client, admission_with(&[117])?).await;
                    assert_eq!(result.is_ok(), fault == 0, "projection fault {fault}");
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity recovery projection thread failed")?
}

fn activity_map(x: i32, handle: u64) -> Result<Vec<u8>, BoxError> {
    use oteryn_protocol_oteryn::world_map::*;
    encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: CONTENT_GENERATION,
            bundle_digest: [2; 32],
            reset_epoch: 7,
            origin: ActorPosition { x, y: 0, floor: 0 },
            first_visible_floor: 0,
        },
        tiles: vec![MapTile {
            position: ActorPosition { x, y: 0, floor: 0 },
            items: vec![MapItem {
                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::Handle(handle.try_into()?),
                appearance_id: 0,
            }],
            more: false,
            ground_speed: 0,
        }],
    })
    .map_err(|error| format!("{error:?}").into())
}

#[test]
fn soulpit_same_generation_entry_and_return_reset_revisions_and_map_handles() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[18, 117],
                        &[18, 117],
                        &[],
                        (1, spatial(0)),
                        &[(17, 23, 1, activity_map(0, 77)?)],
                    )
                    .await?;
                    let frame = read_frame(&mut server).await?;
                    assert_eq!(
                        decode_wire_envelope(&frame)?.client_command(1)?.command_id,
                        7
                    );
                    for (source, destination, sequence, returning) in
                        [(1, 4, 41, false), (4, 5, 3, true)]
                    {
                        let mut changed = receipt(source, destination, 1, returning);
                        changed.selection = vec![18, 117];
                        changed.requires_ack = true;
                        let mut projection = changed.clone();
                        projection.requires_ack = false;
                        transfer(&mut server, 1, sequence, &changed, &projection, 0, true).await?;
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let ack = envelope.client_command(1)?;
                        assert_eq!((ack.command_id, ack.command_type), (1, 710));
                        assert_eq!(
                            activity_scope::decode_activity_scope_ack(ack.payload)
                                .map_err(|e| format!("{e:?}"))?,
                            activity_scope::ActivityScopeAck {
                                handoff: changed.handoff,
                                session: changed.session
                            }
                        );
                        write_frame(
                            &mut server,
                            &encode_command_result(1, 1, 1, CommandStatus::Accepted, &[])?,
                        )
                        .await?;
                        let frame = read_frame(&mut server).await?;
                        assert_eq!(
                            decode_wire_envelope(&frame)?.client_command(1)?.command_id,
                            2
                        );
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                2,
                                2,
                                CommandStatus::Accepted,
                                &encode_step_result(StepDisposition::Blocked),
                            )?,
                        )
                        .await?;
                    }
                    let mut rest = vec![];
                    let _ = server.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[18, 117])?).await?;
                assert_eq!(session.spatial_revision, 5);
                assert!(session.gated_revisions.contains(&(17, 23)));
                assert!(matches!(
                    session.step(StepDirection::North).await,
                    Err(SessionError::ActivityScopeChanged)
                ));
                for (session_id, handle) in [(4_u8, 88_u64), (5, 99)] {
                    assert_eq!(session.connection_generation, 1);
                    assert_eq!(
                        session.join_snapshot.game_session_id.as_bytes(),
                        &uuid_v7(session_id)
                    );
                    assert_eq!(session.spatial_revision, 1);
                    assert_eq!(
                        session
                            .gated_revisions
                            .iter()
                            .find(|(id, _)| *id == 17)
                            .map(|(_, revision)| *revision),
                        Some(1)
                    );
                    let map = session.world_map_snapshot().ok_or("destination map")?;
                    assert_eq!(map.tiles.len(), 1);
                    assert_eq!(
                        map.tiles[0].items[0].origin,
                        oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle.try_into()?)
                    );
                    assert_eq!(session.next_command_id, 2);
                    assert_eq!(session.last_server_sequence, 1);
                    session.take_events();
                    session.step(StepDirection::North).await?;
                    if session_id == 4 {
                        session.wait_for_events(DEADLINE).await?;
                    }
                }
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "same generation activity thread failed")?
}

#[test]
fn soulpit_ack_result_must_be_correlated_before_destination_is_installed() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..4 {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[117], &[117], &[], (1, spatial(0)), &[])
                            .await?;
                        let mut changed = receipt(1, 4, 1, false);
                        changed.requires_ack = true;
                        let mut projection = changed.clone();
                        projection.requires_ack = false;
                        transfer(&mut server, 1, 41, &changed, &projection, 0, true).await?;
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        assert_eq!(envelope.client_command(1)?.command_type, 710);
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                if fault == 0 { 2 } else { 1 },
                                if fault == 1 { 2 } else { 1 },
                                if fault == 2 {
                                    CommandStatus::Rejected
                                } else {
                                    CommandStatus::Accepted
                                },
                                if fault == 3 { &[1] } else { &[] },
                            )?,
                        )
                        .await?;
                        let mut rest = vec![];
                        let _ = server.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[117])?).await?;
                    assert!(
                        session
                            .service_liveness(Duration::from_millis(50))
                            .await
                            .is_err(),
                        "ack fault {fault}"
                    );
                    assert_eq!(
                        session.join_snapshot.game_session_id.as_bytes(),
                        &uuid_v7(1)
                    );
                    assert_eq!(session.world_spatial.actor_position.x, 0);
                    assert!(session.unusable);
                    drop(session);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity ack thread failed")?
}
