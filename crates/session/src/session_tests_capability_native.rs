#[test]
fn p17c_exiva_selection_routes_command_170_and_refuses_without_selection() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[147] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            for (id, payload, reply) in [
                                (7, vec![6, b'A', b'l'], vec![1]),
                                (8, vec![5], vec![2, 2, 0]),
                            ] {
                                let frame = read_frame(&mut server).await?;
                                let envelope = decode_wire_envelope(&frame)?;
                                let command = envelope.client_command(1)?;
                                assert_eq!(
                                    (command.command_type, command.command_id, command.payload),
                                    (170, id, payload.as_slice())
                                );
                                write_frame(
                                    &mut server,
                                    &encode_command_result(
                                        1,
                                        34 + id,
                                        id,
                                        CommandStatus::Accepted,
                                        &reply,
                                    )?,
                                )
                                .await?;
                            }
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut admission = admission()?;
                    admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                    let mut session = Session::admit(client, admission).await?;
                    let result = session.gated_command(147, 170, &[6, b'A', b'l']).await;
                    if selected {
                        assert_eq!(result?.payload, [1]);
                        let result = session.gated_command(147, 170, &[5]).await?;
                        assert_eq!(
                            exiva::PlayerList::decode(&result.payload)
                                .ok_or("list")?
                                .modes,
                            2
                        );
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 147 })
                        ));
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "Exiva selection thread panicked")?
}

#[test]
fn selected_mechanic_panels_have_raw_command_routes() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                // The adapters for CLIENT-E2E-1 all call gated_command, including opt-in panels.
                for (capability, command_type) in [
                    (40, 40),
                    (42, 45),
                    (43, 48),
                    (60, 78),
                    (62, 79),
                    (63, 80),
                    (68, 85),
                    (72, 89),
                    (81, 97),
                    (140, 160),
                    (165, 220),
                    // CL25J: include every newly composed feature route.
                    (151, 177),
                    (152, 181),
                    (153, 182),
                    (154, 183),
                    (179, 348),
                    (195, 378),
                    (197, 383),
                    (199, 387),
                    (225, 431),
                    (230, 440),
                ] {
                    assert!(
                        gated_routes().any(|(id, commands, _)| id == capability
                            && commands.contains(&command_type)),
                        "missing panel route for capability {capability}"
                    );
                }
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer(&mut server, &[40], &[40], &[]).await?;
                    let frame = read_frame(&mut server).await?;
                    assert_eq!(
                        decode_wire_envelope(&frame)?
                            .client_command(1)?
                            .command_type,
                        40
                    );
                    write_frame(
                        &mut server,
                        &encode_command_result(1, 41, 7, CommandStatus::Accepted, b"guild")?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[40])?).await?;
                assert!(session.is_selected(40));
                assert_eq!(session.gated_command(40, 40, b"").await?.payload, b"guild");
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "panel routes thread panicked")?
}

#[test]
fn npc_service_and_staff_prerequisites_route_with_interleaved_domains() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let npc = EntityRef {
                    identity: [9; 16],
                    generation: 2,
                };
                let conversation = npc_service::NpcConversation {
                    open: Some((npc, npc_service::NpcConversationState::Open)),
                    lines: vec!["Trade?".into()],
                };
                let window = npc_service::NpcTradeWindow {
                    open: Some((npc, 7)),
                    offers: vec![npc_service::NpcOffer {
                        item_definition_ref: 5,
                        side: NpcTradeSide::Buy,
                        count: 1,
                        unit_price: 12,
                    }],
                };
                let expected = window.clone();
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        CLIENT_SUPPORTED_CAPABILITIES,
                        &[3, 51],
                        &[],
                        (1, spatial(0)),
                        &[
                            (
                                7,
                                0,
                                1,
                                npc_service::encode_npc_conversation(&Default::default())
                                    .map_err(|e| format!("conversation: {e:?}"))?,
                            ),
                            (
                                8,
                                0,
                                1,
                                npc_service::encode_npc_trade_window(&Default::default())
                                    .map_err(|e| format!("window: {e:?}"))?,
                            ),
                        ],
                    )
                    .await?;
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, 7);
                    assert_eq!(
                        npc_service::decode_npc_talk_intent(command.payload)
                            .map_err(|e| format!("talk: {e:?}"))?
                            .text,
                        "trade"
                    );
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            41,
                            7,
                            CommandStatus::Accepted,
                            &npc_service::encode_npc_intent_result(NpcIntentDisposition::Ok),
                        )?,
                    )
                    .await?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(
                            1,
                            42,
                            7,
                            0,
                            1,
                            1,
                            &npc_service::encode_npc_conversation(&conversation)
                                .map_err(|e| format!("conversation: {e:?}"))?,
                        )?,
                    )
                    .await?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(
                            1,
                            43,
                            8,
                            0,
                            1,
                            1,
                            &npc_service::encode_npc_trade_window(&window)
                                .map_err(|e| format!("window: {e:?}"))?,
                        )?,
                    )
                    .await?;
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, 60);
                    assert_eq!(
                        staff::decode_envelope(command.payload)?.principal,
                        "operator"
                    );
                    let result = staff::StaffResult {
                        code: staff::StaffCode::TierInsufficient,
                        disposition: 2,
                        control_seq: 1,
                        link_sha256: vec![3; 32],
                        mute_until_ms: 0,
                    };
                    write_frame(
                        &mut server,
                        &encode_state_delta(
                            1,
                            44,
                            8,
                            1,
                            2,
                            1,
                            &npc_service::encode_npc_trade_window(&Default::default())
                                .map_err(|e| format!("window: {e:?}"))?,
                        )?,
                    )
                    .await?;
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            45,
                            8,
                            CommandStatus::Accepted,
                            &staff::encode_result(&result)?,
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut admission = admission()?;
                admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                let mut session = Session::admit(client, admission).await?;
                assert!(matches!(
                    session.gated_command(81, 97, b"").await,
                    Err(SessionError::CapabilityNotSelected { capability: 81 })
                ));
                let payload = npc_service::encode_npc_talk_intent(&npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "trade".into(),
                })
                .map_err(|e| format!("talk: {e:?}"))?;
                let result = session.gated_command(3, 7, &payload).await?;
                assert_eq!(
                    npc_service::decode_npc_intent_result(&result.payload)
                        .map_err(|e| format!("result: {e:?}"))?,
                    NpcIntentDisposition::Ok
                );
                let delta = session.read_gated_delta(8, 1).await?;
                assert_eq!(
                    npc_service::decode_npc_trade_window(&delta.payload)
                        .map_err(|e| format!("window: {e:?}"))?,
                    expected
                );
                assert!(matches!(
                    session.take_events().as_slice(),
                    [SessionEvent::NpcConversation(_)]
                ));
                let envelope = staff::SignedEnvelope {
                    principal: "operator".into(),
                    canonical: vec![1],
                    signature: vec![2],
                };
                let result = session
                    .gated_command(51, 60, &staff::encode_envelope(&envelope)?)
                    .await?;
                assert_eq!(
                    staff::decode_result(&result.payload)?.code,
                    staff::StaffCode::TierInsufficient
                );
                assert!(matches!(
                    session.take_events().as_slice(),
                    [SessionEvent::NpcTradeWindow(_)]
                ));
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "NPC session thread panicked")?
}

#[test]
fn world_event_query_uses_selected_session_and_empty_payload() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&53));
                assert!(gated_routes().any(|(id, commands, _)| id == 53 && commands.contains(&64)));
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer(&mut server, &[53], &[53], &[]).await?;
                    let frame = read_frame(&mut server).await?;
                    let command = decode_wire_envelope(&frame)?.client_command(1)?;
                    assert_eq!(command.command_type, 64);
                    assert!(command.payload.is_empty());
                    let payload =
                        world_event::encode_world_event_state(&world_event::WorldEventState {
                            reset_epoch: 9,
                            active: true,
                            announcement: "Fair".into(),
                        })
                        .map_err(|_| "world-event encode failed")?;
                    write_frame(
                        &mut server,
                        &encode_command_result(1, 41, 7, CommandStatus::Accepted, &payload)?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[53])?).await?;
                let result = session.gated_command(53, 64, &[]).await?;
                assert_eq!(
                    world_event::decode_world_event_state(&result.payload)
                        .map_err(|_| "world-event decode failed")?
                        .announcement,
                    "Fair"
                );
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "world-event session test thread panicked")?
}

#[test]
fn highscores_query_uses_selected_session_and_preserves_live_result() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&9));
                let world = highscores::WorldId::decode(&[
                    90, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 90,
                ])?;
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        let caps: &[u32] = if selected { &[9] } else { &[] };
                        join_peer(&mut server, &[9], caps, &[]).await?;
                        if selected {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(command.command_type, 14);
                            let query = highscores::decode_highscores_query(command.payload)
                                .map_err(|e| format!("{e:?}"))?;
                            assert_eq!((query.category, query.page, query.snapshot_id), (1, 1, 0));
                            assert_eq!(query.world, Some(world));
                            assert!(query.details);
                            let reply = highscores::HighscoresResult {
                                world: Some(world),
                                own_page: None,
                                category: 1,
                                snapshot_id: 77,
                                computed_at: 1234,
                                page: 1,
                                has_more: false,
                                rows: vec![],
                                own_row: None,
                            };
                            let payload = highscores::encode_highscores_result(&reply)
                                .map_err(|e| format!("{e:?}"))?;
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    41,
                                    command.command_id,
                                    CommandStatus::Accepted,
                                    &payload,
                                )?,
                            )
                            .await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(&[9])?).await?;
                    let payload =
                        highscores::encode_highscores_query(highscores::HighscoresQuery {
                            world: Some(world),
                            details: true,
                            category: 1,
                            vocation: 0,
                            page: 1,
                            snapshot_id: 0,
                        })
                        .map_err(|e| format!("{e:?}"))?;
                    let result = session.gated_command(9, 14, &payload).await;
                    if selected {
                        let reply = highscores::decode_highscores_result(&result?.payload)
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!((reply.snapshot_id, reply.computed_at), (77, 1234));
                        assert_eq!(reply.world, Some(world));
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 9 })
                        ));
                    }
                    peer.await??;
                }
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "highscores session test panicked")?
}
