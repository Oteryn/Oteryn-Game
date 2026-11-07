//! Requested recovery can prove an unacknowledged committed activity successor.
use super::*;

#[test]
fn srx3_activity_scope_requested_successor_recovery_requires_current_scope_proof()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for fault in 0..6 {
                    let (client, mut peer) = tokio::io::duplex(256 * 1024);
                    let previous = RecoveryState {
                        game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                        world_id: uuid_v7(2),
                        activity_scope: None,
                        connection_generation: 1,
                        next_command_id: 7,
                        last_server_sequence: 40,
                        selected_capabilities: vec![117],
                    };
                    let server = tokio::spawn(async move {
                        let request = read_frame(&mut peer).await?;
                        assert_eq!(
                            decode_wire_envelope(&request)?
                                .client_resume()?
                                .game_session_id,
                            GameSessionId::decode(&uuid_v7(1))?
                        );
                        write_frame(
                            &mut peer,
                            &oteryn_protocol_oteryn::encode_server_resume_accepted(
                                &oteryn_protocol_oteryn::ServerResumeAcceptedValue {
                                    game_session_id: GameSessionId::decode(&uuid_v7(10))?,
                                    connection_generation: 1,
                                    current_server_sequence: if fault == 5 { 2 } else { 1 },
                                    next_command_id: if fault == 4 { 2 } else { 1 },
                                    schema_revision: 1,
                                    selected_capabilities: if fault == 3 {
                                        &[4, 117]
                                    } else {
                                        &[117]
                                    },
                                },
                            )?,
                        )
                        .await?;
                        if fault == 1 {
                            let spatial = spatial(200);
                            let overlay = encode_world_object_overlay_snapshot(&[])
                                .map_err(|e| format!("{e:?}"))?;
                            for frame in encode_single_chunk_snapshot(
                                1,
                                100,
                                1,
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
                        } else {
                            destination_projection(
                                &mut peer,
                                1,
                                200,
                                switch(if fault == 2 { 90 } else { 1 }, 10, 1, false),
                            )
                            .await?;
                        }
                        if fault == 0 {
                            let frame = read_frame(&mut peer).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            assert_eq!(envelope.client_command(1)?.command_id, 1);
                            assert_eq!(
                                envelope.client_command_session_id()?,
                                Some(uuid_v7(10).as_slice())
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
                        }
                        Ok::<(), BoxError>(())
                    });
                    let result = Session::resume(
                        client,
                        previous,
                        Recovery {
                            reconnect_material: b"real-recovery",
                            schema_revision: 1,
                            client_build_id: "srx3-recovery",
                            deadline: DEADLINE,
                            appearance_items: None,
                        },
                    )
                    .await;
                    if fault == 0 {
                        let mut session = result?;
                        assert_eq!(session.next_command_id(), 1);
                        assert_eq!(session.last_server_sequence(), 1);
                        assert_eq!(session.activity_scope(), Some(&switch(1, 10, 1, false)));
                        assert_eq!(
                            session.step(StepDirection::East).await?.disposition,
                            StepDisposition::Blocked
                        );
                        server.await??;
                    } else {
                        assert!(
                            result.is_err(),
                            "unqualified successor recovery fault {fault}"
                        );
                        let _ = server.await?;
                    }
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "activity successor recovery thread panicked")?
}
