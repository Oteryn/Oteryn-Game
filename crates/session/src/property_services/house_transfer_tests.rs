use super::*;
use property_services::house_transfer as wire;

#[test]
fn pc04_house_transfer_handoff_restarts_the_command_namespace() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[59, 86],
                        &[59, 86],
                        &[],
                        (1, spatial(100)),
                        &[],
                    )
                    .await?;
                    for (exit, command_id, sequence) in [(false, 7, 41), (true, 1, 2)] {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!(
                            (command.command_type, command.command_id),
                            (102, command_id)
                        );
                        let intent = wire::decode_house_transfer_intent(command.payload)
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(intent.exit, exit);
                        let ack = wire::HouseTransferAck {
                            transfer: uuid_v7(9),
                            session: uuid_v7(4),
                            world: uuid_v7(2),
                            channel: uuid_v7(3),
                            house: (!exit).then(|| "oteryn:content.house.test".into()),
                            scope_generation: 2,
                            connection_generation: 1,
                            lease_generation: 2,
                            selection: vec![59, 86],
                            position: [100, 0, 0],
                        };
                        let payload =
                            wire::encode_house_transfer_ack(&ack).map_err(|e| format!("{e:?}"))?;
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                sequence,
                                command_id,
                                CommandStatus::Accepted,
                                &payload,
                            )?,
                        )
                        .await?;
                        let spatial_payload = spatial(100);
                        let domain = if exit {
                            DomainSnapshot {
                                domain_id: 1,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &spatial_payload,
                            }
                        } else {
                            DomainSnapshot {
                                domain_id: 51,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &payload,
                            }
                        };
                        for frame in
                            encode_single_chunk_snapshot(1, 1, if exit { 0 } else { 1 }, &[domain])?
                        {
                            write_frame(&mut server, &frame).await?;
                        }
                    }
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!((command.command_type, command.command_id), (77, 1));
                    write_frame(
                        &mut server,
                        &encode_command_result(1, 1, 1, CommandStatus::Rejected, &[])?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut input = admission()?;
                input.supported_capabilities = &[59, 86];
                let mut session = Session::admit(client, input).await?;
                for exit in [false, true] {
                    let payload = wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
                        exit,
                        house: "oteryn:content.house.test".into(),
                    })
                    .map_err(|e| format!("{e:?}"))?;
                    session.gated_command(86, 102, &payload).await?;
                    assert_eq!(session.next_command_id, 1);
                    assert_eq!(session.last_server_sequence, if exit { 0 } else { 1 });
                }
                let query = property_services::house_move_out::encode_house_move_out_request(
                    &property_services::house_move_out::HouseMoveOutRequest {
                        house: "oteryn:content.house.test".into(),
                        action: 0,
                        due_at_unix_ms: 0,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    session.gated_command(59, 77, &query).await?.status,
                    CommandStatus::Rejected
                );
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "handoff test thread failed")?
}

#[test]
fn pc04_house_transfer_codec_refuses_malformed_and_roundtrips_boundaries() -> Result<(), BoxError> {
    let ack = wire::HouseTransferAck {
        transfer: uuid_v7(9),
        session: uuid_v7(4),
        world: uuid_v7(2),
        channel: uuid_v7(3),
        house: Some("oteryn:content.house.test".into()),
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![86],
        position: [i32::MIN, i32::MAX, -8],
    };
    let payload = wire::encode_house_transfer_ack(&ack).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        wire::decode_house_transfer_ack(&payload).map_err(|e| format!("{e:?}"))?,
        ack
    );
    for length in 0..payload.len() {
        assert!(wire::decode_house_transfer_ack(&payload[..length]).is_err());
    }
    let mut duplicate = payload.clone();
    duplicate.extend([48, 1]);
    assert!(wire::decode_house_transfer_ack(&duplicate).is_err());
    let mut overflow = payload;
    overflow.extend([96, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2]);
    assert!(wire::decode_house_transfer_ack(&overflow).is_err());
    assert!(wire::decode_house_transfer_ack(&vec![0; wire::MAX_ACK_BYTES + 1]).is_err());
    assert!(
        wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
            exit: false,
            house: "invalid".into()
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn pc04_transfer_refusal_preserves_namespace_and_invalid_ack_poison() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for invalid in [false, true] {
                    let (client, mut server) = tokio::io::duplex(128 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[86], &[86], &[], (1, spatial(100)), &[])
                            .await?;
                        read_frame(&mut server).await?;
                        let reply = if invalid {
                            // A valid receipt for another destination must never change this session.
                            wire::encode_house_transfer_ack(&wire::HouseTransferAck {
                                transfer: uuid_v7(9),
                                session: uuid_v7(4),
                                world: uuid_v7(2),
                                channel: uuid_v7(3),
                                house: Some("oteryn:content.house.other".into()),
                                scope_generation: 2,
                                connection_generation: 1,
                                lease_generation: 2,
                                selection: vec![86],
                                position: [100, 0, 0],
                            })
                            .map_err(|e| format!("{e:?}"))?
                        } else {
                            vec![]
                        };
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                41,
                                7,
                                if invalid {
                                    CommandStatus::Accepted
                                } else {
                                    CommandStatus::Rejected
                                },
                                &reply,
                            )?,
                        )
                        .await?;
                        Ok::<(), BoxError>(())
                    });
                    let mut input = admission()?;
                    input.supported_capabilities = &[86];
                    let mut session = Session::admit(client, input).await?;
                    let original = session.join_snapshot.game_session_id;
                    let request = wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
                        exit: false,
                        house: "oteryn:content.house.test".into(),
                    })
                    .map_err(|e| format!("{e:?}"))?;
                    let result = session.gated_command(86, 102, &request).await;
                    if invalid {
                        assert!(result.is_err());
                        assert!(session.ensure_usable().is_err());
                    } else {
                        assert_eq!(result?.status, CommandStatus::Rejected);
                        assert!(session.ensure_usable().is_ok());
                    }
                    assert_eq!(session.next_command_id, 8);
                    assert_eq!(session.last_server_sequence, 41);
                    assert_eq!(session.join_snapshot.game_session_id, original);
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "refusal test thread failed")?
}

#[test]
fn pc04_property_codecs_use_the_real_session_routes() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                use property_services::{guildhall, house_move_out as move_out};
                let (client, mut server) = tokio::io::duplex(128 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[40, 49, 59, 99],
                        &[40, 49, 59, 99],
                        &[],
                        (1, spatial(100)),
                        &[],
                    )
                    .await?;
                    for (index, kind) in [76, 77, 115].into_iter().enumerate() {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!(
                            (command.command_type, command.command_id),
                            (kind, 7 + index as u64)
                        );
                        let reply = if kind == 115 {
                            let intent = guildhall::decode_guildhall_leadership(command.payload)
                                .map_err(|e| format!("{e:?}"))?;
                            assert_eq!(intent.expected_revision, 1);
                            guildhall::encode_guild_intent_result(&guildhall::GuildIntentResult {
                                code: 1,
                                guild: intent.guild,
                                revision: Some(2),
                            })
                            .map_err(|e| format!("{e:?}"))?
                        } else {
                            let request = move_out::decode_house_move_out_request(command.payload)
                                .map_err(|e| format!("{e:?}"))?;
                            assert_eq!(request.action, if kind == 76 { 2 } else { 0 });
                            move_out::encode_house_move_out_reply(&move_out::HouseMoveOutReply {
                                operation: None,
                                state: 0,
                                due_at_unix_ms: 0,
                                census_count: 0,
                                delivered: 0,
                            })
                            .map_err(|e| format!("{e:?}"))?
                        };
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                41 + index as u64,
                                7 + index as u64,
                                CommandStatus::Accepted,
                                &reply,
                            )?,
                        )
                        .await?;
                    }
                    Ok::<(), BoxError>(())
                });
                let mut input = admission()?;
                input.supported_capabilities = &[40, 49, 59, 99];
                let mut session = Session::admit(client, input).await?;
                for (kind, action) in [(76, 2), (77, 0)] {
                    let payload =
                        move_out::encode_house_move_out_request(&move_out::HouseMoveOutRequest {
                            house: "oteryn:content.house.test".into(),
                            action,
                            due_at_unix_ms: 0,
                        })
                        .map_err(|e| format!("{e:?}"))?;
                    let result = session.gated_command(59, kind, &payload).await?;
                    assert_eq!(
                        move_out::decode_house_move_out_reply(&result.payload)
                            .map_err(|e| format!("{e:?}"))?
                            .state,
                        0
                    );
                }
                let payload =
                    guildhall::encode_guildhall_leadership(guildhall::GuildhallLeadershipIntent {
                        guild: uuid_v7(4),
                        successor: uuid_v7(5),
                        expected_revision: 1,
                    })
                    .map_err(|e| format!("{e:?}"))?;
                let result = session.gated_command(99, 115, &payload).await?;
                assert_eq!(
                    guildhall::decode_guild_intent_result(&result.payload)
                        .map_err(|e| format!("{e:?}"))?
                        .revision,
                    Some(2)
                );
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "property routes test thread failed")?
}
