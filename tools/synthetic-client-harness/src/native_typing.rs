//! C38: composer state -> native adapters -> admitted synthetic TLS client -> wire.
use super::*;
use oteryn_client_runtime::player_typing as native_typing;
use oteryn_client_ui::player_typing::{self as ui, PlayerTypingPanel};
use oteryn_protocol_oteryn::{chat, player_typing};
use std::time::Instant;

async fn typing_server(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
) -> Result<(), BoxError> {
    let (tcp, _) = listener.accept().await?;
    let mut stream = acceptor.accept(tcp).await?;
    decode_wire_envelope(&read_frame(&mut stream).await?)?.client_bootstrap()?;
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: GENERATION,
            current_server_sequence: JOIN_SEQUENCE,
            next_command_id: FIRST_COMMAND_ID,
            schema_revision: 1,
            selected_capabilities: &[6, 7, 233],
        })?,
    )
    .await?;
    let entities =
        encode_world_spatial_entities_snapshot_with_item_handles(&WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: position(0, 0),
            own_identity: OWN_IDENTITY,
            entities: vec![actor_entity(EntityKind::Player, 0, position(0, 0))],
        })
        .map_err(|e| format!("entities: {e:?}"))?;
    let overlay =
        encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("overlay: {e:?}"))?;
    let rooms = chat::encode_chat_rooms(ChatRoomSet::default());
    send(
        &mut stream,
        &encode_single_chunk_snapshot(
            GENERATION,
            1,
            JOIN_SEQUENCE,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &entities,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: chat::STATE_DOMAIN_CHAT,
                    revision: 1,
                    snapshot_type: chat::SNAPSHOT_TYPE_CHAT_V1,
                    payload: &rooms,
                },
            ],
        )?,
    )
    .await?;
    // No extra start may leak through the rate gate or duplicate an unchanged edit.
    for (offset, typing) in [true, false, true, false].into_iter().enumerate() {
        let (id, kind, payload) = read_command(&mut stream).await?;
        assert_eq!(id, FIRST_COMMAND_ID + offset as u64);
        assert_eq!(kind, player_typing::COMMAND_TYPE_PLAYER_TYPING);
        assert_eq!(player_typing::decode_intent(&payload), Ok(typing));
        send(
            &mut stream,
            &[encode_command_result(
                GENERATION,
                JOIN_SEQUENCE + offset as u64 + 1,
                id,
                CommandStatus::Accepted,
                &[],
            )?],
        )
        .await?;
    }
    let (id, kind, payload) = read_command(&mut stream).await?;
    assert_eq!(id, FIRST_COMMAND_ID + 4);
    assert_eq!(kind, chat::COMMAND_TYPE_CHAT_INTENT);
    assert_eq!(
        chat::decode_chat_intent(&payload).map_err(|e| format!("chat intent: {e:?}"))?,
        ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "hello".into(),
        }
    );
    let result = chat::encode_chat_intent_result(&chat::ChatIntentResult {
        disposition: ChatDisposition::Ok,
        wait_seconds: 0,
    })
    .map_err(|e| format!("chat result: {e:?}"))?;
    send(
        &mut stream,
        &[encode_command_result(
            GENERATION,
            JOIN_SEQUENCE + 5,
            id,
            CommandStatus::Accepted,
            &result,
        )?],
    )
    .await?;
    let mut rest = Vec::new();
    // Dropping the admitted client closes TCP without a TLS close_notify.
    if let Err(error) =
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut rest)).await?
        && error.kind() != std::io::ErrorKind::UnexpectedEof
    {
        return Err(error.into());
    }
    assert!(rest.is_empty());
    Ok(())
}

#[test]
fn native_typing_admitted_client_retries_rate_gate_and_stops_on_send() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (certificate, acceptor, listener) = tls_listener().await?;
                    let address = listener.local_addr()?;
                    let server = tokio::spawn(typing_server(listener, acceptor));
                    let mut client = connect_session(JoinRequest {
                        address,
                        server_name: "localhost",
                        root_certificate: &certificate,
                        schema_revision: 1,
                        character_id: CharacterId::decode(&uuid_v7(4))?,
                        admission_material: b"fixture-grant",
                        client_build_id: "c38-harness",
                        deadline: Duration::from_secs(5),
                    })
                    .await?;
                    let mut panel = PlayerTypingPanel::default();
                    panel.initialize(client.selected_capabilities().contains(&233));
                    let now = Instant::now();
                    let start = ui::composer(&mut panel, true, "h", now).ok_or("start")?;
                    let response = native_typing::send(client.session_mut(), &start).await?;
                    ui::apply_result(&mut panel, start, response);
                    assert!(panel.own_typing);
                    let stop = ui::composer(&mut panel, false, "h", now).ok_or("blur")?;
                    let response = native_typing::send(client.session_mut(), &stop).await?;
                    ui::apply_result(&mut panel, stop, response);
                    assert!(ui::composer(&mut panel, true, "hello", now).is_none());
                    assert!(ui::tick(&mut panel, now + Duration::from_millis(499)).is_none());
                    let retry = ui::tick(&mut panel, now + Duration::from_millis(500))
                        .ok_or("retry wanted start")?;
                    let response = native_typing::send(client.session_mut(), &retry).await?;
                    ui::apply_result(&mut panel, retry, response);
                    assert!(
                        ui::composer(&mut panel, true, "hello", now + Duration::from_millis(501))
                            .is_none()
                    );
                    let mut chat = oteryn_client_ui::chat::ChatPanel {
                        selected: true,
                        text: "hello".into(),
                        ..Default::default()
                    };
                    let intent = oteryn_client_ui::chat::request(
                        &mut chat,
                        oteryn_client_ui::chat::Mode::Say,
                        "",
                    )
                    .ok_or("send chat")?;
                    let stop = ui::composer(
                        &mut panel,
                        false,
                        &chat.text,
                        now + Duration::from_millis(501),
                    )
                    .ok_or("stop on send")?;
                    let response = native_typing::send(client.session_mut(), &stop).await?;
                    ui::apply_result(&mut panel, stop, response);
                    assert!(!panel.own_typing);
                    assert_eq!(client.chat(&intent).await?.status, CommandStatus::Accepted);
                    drop(client);
                    server.await??;
                    Ok::<(), BoxError>(())
                })
        })?
        .join()
        .map_err(|_| "typing scenario thread panicked")?
}
