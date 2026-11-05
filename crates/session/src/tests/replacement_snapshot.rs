use super::*;
use oteryn_protocol_oteryn::world_map::*;

fn map(floor: i16, epoch: u64) -> WorldMapViewSnapshot {
    WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: CONTENT_GENERATION,
            bundle_digest: [2; 32],
            reset_epoch: epoch,
            origin: ActorPosition {
                x: 100,
                y: 100,
                floor,
            },
            first_visible_floor: first_visible_floor_start(floor),
        },
        tiles: vec![],
    }
}

#[test]
fn replacement_snapshot_idle_and_during_command_continues_sequence() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for in_flight in [false, true] {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(async move {
                        let initial =
                            encode_world_map_snapshot(&map(0, 1)).map_err(|e| format!("{e:?}"))?;
                        join_peer_with(
                            &mut server,
                            &[18],
                            &[18],
                            &[],
                            (1, spatial(100)),
                            &[(17, 1, 1, initial)],
                        )
                        .await?;
                        if in_flight {
                            read_frame(&mut server).await?;
                        }
                        let replacement =
                            encode_world_map_snapshot(&map(-8, 2)).map_err(|e| format!("{e:?}"))?;
                        let vitals = actor_spell::encode_actor_vitals(&vitals(77, 55))
                            .map_err(|e| format!("{e:?}"))?;
                        for frame in encode_single_chunk_snapshot(
                            1,
                            2,
                            41,
                            &[
                                DomainSnapshot {
                                    domain_id: 17,
                                    revision: 7,
                                    snapshot_type: 1,
                                    payload: &replacement,
                                },
                                DomainSnapshot {
                                    domain_id: 3,
                                    revision: 9,
                                    snapshot_type: 1,
                                    payload: &vitals,
                                },
                            ],
                        )? {
                            write_frame(&mut server, &frame).await?;
                        }
                        if !in_flight {
                            read_frame(&mut server).await?;
                        }
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                42,
                                7,
                                CommandStatus::Accepted,
                                &encode_step_result(StepDisposition::Blocked),
                            )?,
                        )
                        .await?;
                        let changed = encode_world_map_delta(
                            &map(-8, 2).header,
                            &WorldMapViewDelta {
                                header: MapViewHeader {
                                    origin: ActorPosition {
                                        x: 101,
                                        ..map(-8, 2).header.origin
                                    },
                                    ..map(-8, 2).header
                                },
                                tiles: vec![],
                                cleared: vec![],
                            },
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        write_frame(
                            &mut server,
                            &encode_state_delta(1, 43, 17, 7, 8, 1, &changed)?,
                        )
                        .await?;
                        let mut rest = vec![];
                        let _ = server.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[18])?).await?;
                    if !in_flight {
                        session.wait_for_events(DEADLINE).await?;
                    }
                    session.step(StepDirection::North).await?;
                    assert_eq!(session.world_map_snapshot(), Some(map(-8, 2)));
                    assert_eq!(session.actor_vitals(), Some(&vitals(77, 55)));
                    let events = session.wait_for_events(DEADLINE).await?;
                    // Already queued replacement events may return before the following delta.
                    if !events
                        .iter()
                        .any(|event| matches!(event, SessionEvent::WorldMap(_)))
                    {
                        session.wait_for_events(DEADLINE).await?;
                    }
                    assert_eq!(session.last_server_sequence, 43);
                    drop(session);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "replacement thread failed")?
}

#[test]
fn replacement_snapshot_is_atomic_and_poisoned_on_invalid_transfer_or_domain()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..4 {
                    let (client, mut server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(async move {
                        let initial =
                            encode_world_map_snapshot(&map(0, 1)).map_err(|e| format!("{e:?}"))?;
                        join_peer_with(
                            &mut server,
                            &[18],
                            &[18],
                            &[],
                            (1, spatial(100)),
                            &[(17, 1, 1, initial)],
                        )
                        .await?;
                        let replacement =
                            encode_world_map_snapshot(&map(-8, 2)).map_err(|e| format!("{e:?}"))?;
                        let domains = [
                            DomainSnapshot {
                                domain_id: 17,
                                revision: 7,
                                snapshot_type: 1,
                                payload: &replacement,
                            },
                            DomainSnapshot {
                                domain_id: 3,
                                revision: 9,
                                snapshot_type: 1,
                                payload: b"invalid vitals",
                            },
                        ];
                        let mut frames = encode_single_chunk_snapshot(
                            1,
                            2,
                            if fault == 2 { 40 } else { 41 },
                            if fault == 3 { &domains } else { &domains[..1] },
                        )?;
                        if fault == 0 {
                            frames[2] =
                                encode_single_chunk_snapshot(1, 3, 41, &domains[..1])?[2].clone();
                        }
                        if fault == 1 {
                            frames[1] =
                                encode_single_chunk_snapshot(2, 2, 41, &domains[..1])?[1].clone();
                        }
                        for frame in frames {
                            write_frame(&mut server, &frame).await?;
                        }
                        let mut rest = vec![];
                        let _ = server.read_to_end(&mut rest).await;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[18])?).await?;
                    assert!(session.wait_for_events(DEADLINE).await.is_err());
                    assert_eq!(session.world_map_snapshot(), Some(map(0, 1)));
                    assert_eq!(session.last_server_sequence, 40);
                    assert_eq!(session.actor_vitals(), None);
                    assert!(session.take_events().is_empty());
                    assert!(matches!(
                        session.step(StepDirection::North).await,
                        Err(SessionError::SessionUnusable)
                    ));
                    drop(session);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "invalid replacement thread failed")?
}

#[test]
fn replacement_snapshot_multichunk_replaces_gated_domain_and_rebases_waiting_delta()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer(&mut server, &[1], &[1], &[(GATED_DOMAIN, 1)]).await?;
                    let payload = vec![0x55; oteryn_protocol_oteryn::MAX_SNAPSHOT_CHUNK_BYTES + 1];
                    let frames = oteryn_protocol_oteryn::encode_snapshot(
                        1,
                        2,
                        41,
                        &[DomainSnapshot {
                            domain_id: GATED_DOMAIN,
                            revision: 9,
                            snapshot_type: 1,
                            payload: &payload,
                        }],
                    )?;
                    assert!(frames.len() > 3);
                    for frame in frames {
                        write_frame(&mut server, &frame).await?;
                    }
                    write_frame(
                        &mut server,
                        &encode_state_delta(1, 42, GATED_DOMAIN, 9, 10, 1, b"continued")?,
                    )
                    .await?;
                    let mut rest = vec![];
                    let _ = server.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[1])?).await?;
                let delta = session.read_gated_delta(GATED_DOMAIN, 1).await?;
                assert_eq!((delta.base_revision, delta.new_revision), (9, 10));
                assert_eq!(delta.payload, b"continued");
                assert_eq!(
                    session
                        .gated_snapshot(GATED_DOMAIN)
                        .map(|s| (s.revision, s.payload.len())),
                    Some((9, oteryn_protocol_oteryn::MAX_SNAPSHOT_CHUNK_BYTES + 1))
                );
                assert_eq!(session.last_server_sequence, 42);
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "multichunk replacement thread failed")?
}

#[test]
fn replacement_snapshot_between_result_and_claimed_delta_rebases_spatial_state()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    let initial =
                        encode_world_map_snapshot(&map(0, 1)).map_err(|e| format!("{e:?}"))?;
                    join_peer_with(
                        &mut server,
                        &[18],
                        &[18],
                        &[],
                        (1, spatial(100)),
                        &[(17, 1, 1, initial)],
                    )
                    .await?;
                    read_frame(&mut server).await?;
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            41,
                            7,
                            CommandStatus::Accepted,
                            &encode_step_result(StepDisposition::Moved),
                        )?,
                    )
                    .await?;
                    let replacement =
                        encode_world_map_snapshot(&map(-8, 2)).map_err(|e| format!("{e:?}"))?;
                    let observation = encode_world_spatial(&WireSpatialObservation {
                        content_generation: CONTENT_GENERATION,
                        actor_position: map(-8, 2).header.origin,
                    });
                    for frame in encode_single_chunk_snapshot(
                        1,
                        2,
                        42,
                        &[
                            DomainSnapshot {
                                domain_id: 1,
                                revision: 11,
                                snapshot_type: 1,
                                payload: &observation,
                            },
                            DomainSnapshot {
                                domain_id: 17,
                                revision: 7,
                                snapshot_type: 1,
                                payload: &replacement,
                            },
                        ],
                    )? {
                        write_frame(&mut server, &frame).await?;
                    }
                    write_frame(
                        &mut server,
                        &encode_state_delta(1, 43, 1, 11, 12, 1, &observation)?,
                    )
                    .await?;
                    let mut rest = vec![];
                    let _ = server.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[18])?).await?;
                let outcome = session.step(StepDirection::North).await?;
                let delta = outcome.world_spatial_delta.ok_or("missing claimed delta")?;
                assert_eq!((delta.base_revision, delta.new_revision), (11, 12));
                assert_eq!(delta.value.actor_position, map(-8, 2).header.origin);
                assert_eq!(session.world_map_snapshot(), Some(map(-8, 2)));
                assert_eq!(session.last_server_sequence, 43);
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "in-flight replacement thread failed")?
}

#[test]
fn replacement_snapshot_notifies_overlay_and_gated_consumers() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer(&mut server, &[1], &[1], &[(GATED_DOMAIN, 1)]).await?;
                    let overlay =
                        encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
                    for frame in encode_single_chunk_snapshot(
                        1,
                        2,
                        41,
                        &[
                            DomainSnapshot {
                                domain_id: 2,
                                revision: 9,
                                snapshot_type: 1,
                                payload: &overlay,
                            },
                            DomainSnapshot {
                                domain_id: GATED_DOMAIN,
                                revision: 12,
                                snapshot_type: 1,
                                payload: b"replacement",
                            },
                        ],
                    )? {
                        write_frame(&mut server, &frame).await?;
                    }
                    let mut rest = vec![];
                    let _ = server.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[1])?).await?;
                let events = session.wait_for_events(DEADLINE).await?;
                let replaced = events
                    .iter()
                    .filter_map(|event| match event {
                        SessionEvent::GatedSnapshotReplaced(snapshot) => {
                            Some((snapshot.value.domain_id, snapshot.value.revision))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(replaced, vec![(2, 9), (GATED_DOMAIN, 12)]);
                assert!(events.iter().any(|event| matches!(
                    event,
                    SessionEvent::WorldObjectOverlaySnapshot(snapshot) if snapshot.value.is_empty()
                )));
                assert!(
                    events.iter().any(|event| event.domain_id() == 2),
                    "overlay replacement was silent"
                );
                assert!(
                    events.iter().any(|event| event.domain_id() == GATED_DOMAIN),
                    "gated replacement was silent"
                );
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "replacement notification thread failed")?
}
