#[test]
fn recovery_preserves_identity_sequences_and_selection() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..7 {
                    let (client, mut server) = tokio::io::duplex(16384);
                    let previous = RecoveryState {
                        game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                        world_id: uuid_v7(2),
                        activity_scope: None,
                        connection_generation: 1,
                        next_command_id: 7,
                        last_server_sequence: 40,
                        selected_capabilities: vec![],
                    };
                    let peer = tokio::spawn(async move {
                        let request = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&request)?;
                        let resume = envelope.client_resume()?;
                        assert_eq!(resume.game_session_id, GameSessionId::decode(&uuid_v7(1))?);
                        assert_eq!(resume.last_applied_server_sequence, 40);
                        assert_eq!(resume.reconnect_material, b"recovery");
                        let accepted =
                            oteryn_protocol_oteryn::encode_server_resume_accepted(
                                &oteryn_protocol_oteryn::ServerResumeAcceptedValue {
                                    game_session_id: GameSessionId::decode(&uuid_v7(
                                        if fault == 1 { 2 } else { 1 },
                                    ))?,
                                    connection_generation: if fault == 2 { 1 } else { 2 },
                                    current_server_sequence: if fault == 3 { 39 } else { 42 },
                                    next_command_id: if fault == 4 { 6 } else { 8 },
                                    schema_revision: 1,
                                    selected_capabilities: if fault == 5 { &[21] } else { &[] },
                                },
                            )?;
                        write_frame(&mut server, &accepted).await?;
                        if (1..=5).contains(&fault) {
                            return Ok::<_, BoxError>(());
                        }
                        let spatial = spatial(1);
                        let overlay = encode_world_object_overlay_snapshot(&[])
                            .map_err(|e| format!("{e:?}"))?;
                        for frame in encode_single_chunk_snapshot(
                            2,
                            2,
                            if fault == 6 { 41 } else { 42 },
                            &[
                                DomainSnapshot {
                                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                                    revision: 6,
                                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                                    payload: &spatial,
                                },
                                DomainSnapshot {
                                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                                    revision: 3,
                                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                                    payload: &overlay,
                                },
                            ],
                        )? {
                            write_frame(&mut server, &frame).await?;
                            if fault == 6 {
                                // The peer rejects SnapshotBegin; do not race its close with chunks.
                                return Ok(());
                            }
                        }
                        if fault == 0 {
                            let frame = read_frame(&mut server).await?;
                            assert_eq!(
                                decode_wire_envelope(&frame)?.client_command(2)?.command_id,
                                8
                            );
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    2,
                                    43,
                                    8,
                                    CommandStatus::Accepted,
                                    &encode_step_result(StepDisposition::Blocked),
                                )?,
                            )
                            .await?;
                        }
                        Ok(())
                    });
                    let result = Session::resume(
                        client,
                        previous,
                        Recovery {
                            reconnect_material: b"recovery",
                            schema_revision: 1,
                            client_build_id: "recovery-test",
                            deadline: DEADLINE,
                            appearance_items: None,
                        },
                    )
                    .await;
                    if fault == 0 {
                        let mut resumed = result?;
                        assert_eq!(resumed.next_command_id(), 8);
                        assert_eq!(resumed.last_server_sequence(), 42);
                        assert_eq!(resumed.world_spatial().actor_position.x, 1);
                        assert_eq!(
                            resumed.step(StepDirection::East).await?.disposition,
                            StepDisposition::Blocked
                        );
                    } else {
                        assert!(result.is_err(), "recovery accepted fault {fault}");
                    }
                    peer.await??;
                }
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "recovery test thread failed")?
}

#[test]
fn recovery_retains_unacknowledged_step_id_after_flushing_command() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(16384);
                let peer = tokio::spawn(async move {
                    join_peer_with(&mut server, &[13], &[], &[], (1, spatial(0)), &[]).await?;
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_id, 7);
                    assert_eq!(
                        command.command_type,
                        world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
                    );
                    assert_eq!(
                        world_spatial::decode_step_intent(command.payload)
                            .map_err(|error| format!("step payload: {error:?}"))?,
                        StepDirection::East
                    );
                    Ok::<_, BoxError>(())
                });
                let session = Session::admit(client, admission()?).await?;
                let pending = session.disconnect_after_step(StepDirection::East).await?;
                assert_eq!(pending.next_command_id, 7);
                assert_eq!(pending.last_server_sequence, 40);
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "pending recovery thread failed")?
}
