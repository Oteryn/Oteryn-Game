#[test]
fn recovery_connects_new_tls_transport_and_keeps_session() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (certificate, acceptor, listener, address) = tls_test_listener().await?;
                let recovery_listener = TcpListener::bind("127.0.0.1:0").await?;
                let recovery_address = recovery_listener.local_addr()?;
                let recovery_acceptor = acceptor.clone();
                let first = tokio::spawn(run_fake_server(listener, acceptor));
                let client = connect_session(JoinRequest {
                    address,
                    server_name: "localhost",
                    root_certificate: &certificate,
                    schema_revision: 1,
                    character_id: CharacterId::decode(&test_uuid_v7(4))?,
                    admission_material: b"fixture-grant",
                    client_build_id: CLIENT_BUILD_ID,
                    deadline: TEST_DEADLINE,
                })
                .await?;
                let id = client.join_snapshot().game_session_id;
                let previous = client.disconnect();
                first.await??;
                let peer = tokio::spawn(async move {
                    let (tcp, _) = recovery_listener.accept().await?;
                    let mut stream = recovery_acceptor.accept(tcp).await?;
                    let frame = read_frame(&mut stream).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let request = envelope.client_resume()?;
                    assert_eq!(request.game_session_id, id);
                    assert_eq!(request.reconnect_material, b"recovery");
                    write_frame(
                        &mut stream,
                        &oteryn_protocol_oteryn::encode_server_resume_accepted(
                            &oteryn_protocol_oteryn::ServerResumeAcceptedValue {
                                game_session_id: id,
                                connection_generation: 2,
                                current_server_sequence: 0,
                                next_command_id: 1,
                                schema_revision: 1,
                                selected_capabilities: &[],
                            },
                        )?,
                    )
                    .await?;
                    let spatial = spatial_payload(0, 0, CONTENT_GENERATION);
                    let overlay =
                        encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
                    for frame in encode_single_chunk_snapshot(
                        2,
                        2,
                        0,
                        &[
                            DomainSnapshot {
                                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                                revision: 1,
                                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                                payload: &spatial,
                            },
                            DomainSnapshot {
                                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                                revision: 0,
                                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                                payload: &overlay,
                            },
                        ],
                    )? {
                        write_frame(&mut stream, &frame).await?;
                    }
                    let frame = read_frame(&mut stream).await?;
                    assert_eq!(
                        decode_wire_envelope(&frame)?.client_command(2)?.command_id,
                        1
                    );
                    write_frame(
                        &mut stream,
                        &oteryn_protocol_oteryn::encode_command_result(
                            2,
                            1,
                            1,
                            CommandStatus::Accepted,
                            &world_spatial::encode_step_result(StepDisposition::Blocked),
                        )?,
                    )
                    .await?;
                    Ok::<_, BoxError>(())
                });
                let mut recovered = resume_session(
                    RecoveryTransport {
                        address: recovery_address,
                        server_name: "localhost",
                        root_certificate: &certificate,
                        deadline: TEST_DEADLINE,
                    },
                    previous,
                    Recovery {
                        reconnect_material: b"recovery",
                        schema_revision: 1,
                        client_build_id: CLIENT_BUILD_ID,
                        deadline: TEST_DEADLINE,
                        appearance_items: None,
                    },
                )
                .await?;
                assert_eq!(recovered.join_snapshot().game_session_id, id);
                assert_eq!(
                    recovered.step(StepDirection::East).await?.disposition,
                    StepDisposition::Blocked
                );
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "recovery test thread failed")?
}
