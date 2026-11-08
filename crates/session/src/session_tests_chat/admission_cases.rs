use super::*;

#[test]
fn the_join_snapshot_sets_the_open_rooms_or_leaves_them_empty() -> Result<(), BoxError> {
    block_on(async {
        let open = room_set(&[ChatRoom::World, ChatRoom::Help]);
        let (client, peer) = chat_peer(vec![chat_domain(3, open)], vec![]);
        let session = Session::admit(client, chat_admission()?).await?;
        let log = session.chat_log().ok_or("capability 7 selected")?;
        assert_eq!(log.rooms(), open);
        assert!(log.is_empty());
        drop(session);
        peer.await??;

        // Selected without a domain-12 entry: no rooms are open.
        let (client, peer) = chat_peer(vec![], vec![]);
        let session = Session::admit(client, chat_admission()?).await?;
        assert_eq!(
            session.chat_log().map(ChatLog::rooms),
            Some(ChatRoomSet::default())
        );
        drop(session);
        peer.await??;

        // Unselected: no chat state at all.
        let (client, peer) = script_peer(vec![]);
        let session = Session::admit(client, admission()?).await?;
        assert!(session.chat_log().is_none());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_chat_snapshot_of_another_type_or_without_the_capability_is_refused() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = chat_peer(
            vec![(
                STATE_DOMAIN_CHAT,
                3,
                2,
                chat::encode_chat_rooms(ChatRoomSet::default()),
            )],
            vec![],
        );
        let result = Session::admit(client, chat_admission()?).await;
        drop(peer);
        assert!(matches!(
            result,
            Err(SessionError::UnregisteredSnapshotType {
                domain_id: STATE_DOMAIN_CHAT,
                snapshot_type: 2
            })
        ));

        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[13],
                &[13],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                &[chat_domain(3, ChatRoomSet::default())],
            )
            .await
        });
        let result = Session::admit(client, admission()?).await;
        let _ = peer.await?;
        assert!(matches!(
            result,
            Err(SessionError::UnselectedDomain {
                domain_id: STATE_DOMAIN_CHAT
            })
        ));
        Ok(())
    })?
}

#[test]
fn every_intent_variant_reaches_the_wire_at_the_text_bound() -> Result<(), BoxError> {
    block_on(async {
        let text = "x".repeat(MAX_CHAT_TEXT_BYTES);
        let name = "n".repeat(MAX_CHAT_NAME_BYTES);
        let intents = vec![
            ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: text.clone(),
            },
            ChatIntent::Say {
                mode: ChatSpeechMode::Whisper,
                text: text.clone(),
            },
            ChatIntent::Say {
                mode: ChatSpeechMode::Yell,
                text: text.clone(),
            },
            ChatIntent::Private {
                recipient_name: name,
                text: text.clone(),
            },
            ChatIntent::Room {
                room: ChatRoom::Advertising,
                text,
            },
            ChatIntent::OpenRoom(ChatRoom::English),
            ChatIntent::CloseRoom(ChatRoom::English),
        ];
        let expected = intents.clone();
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[CAPABILITY_CHAT_V1, 13],
                &[CAPABILITY_CHAT_V1, 13],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                &[],
            )
            .await?;
            for (offset, intent) in (0_u64..).zip(&expected) {
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let command = envelope.client_command(1)?;
                assert_eq!(command.command_type, COMMAND_TYPE_CHAT_INTENT);
                assert_eq!(command.command_id, 7 + offset);
                assert_eq!(
                    &chat::decode_chat_intent(command.payload)
                        .map_err(|error| format!("{error:?}"))?,
                    intent
                );
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        41 + offset,
                        7 + offset,
                        CommandStatus::Accepted,
                        &chat::encode_chat_intent_result(&chat::ChatIntentResult {
                            disposition: ChatDisposition::Ok,
                            wait_seconds: 0,
                        })
                        .map_err(|error| format!("{error:?}"))?,
                    )?,
                )
                .await?;
            }
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, chat_admission()?).await?;
        for (offset, intent) in (0_u64..).zip(&intents) {
            let outcome = session.chat(intent).await?;
            assert_eq!(
                (
                    outcome.command_id,
                    outcome.status,
                    outcome.disposition,
                    outcome.wait_seconds,
                    outcome.result_server_sequence
                ),
                (
                    7 + offset,
                    CommandStatus::Accepted,
                    ChatDisposition::Ok,
                    0,
                    41 + offset
                )
            );
        }
        // An OK result applies nothing itself: the line and room change arrive as deltas.
        assert!(session.take_events().is_empty());
        assert!(session.chat_log().is_some_and(ChatLog::is_empty));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn an_intent_over_the_bound_is_refused_before_anything_is_sent() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = chat_peer(vec![], vec![]);
        let mut session = Session::admit(client, chat_admission()?).await?;
        let too_long = ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "x".repeat(MAX_CHAT_TEXT_BYTES + 1),
        };
        assert!(matches!(
            session.chat(&too_long).await,
            Err(SessionError::Chat(ChatWireError::LimitExceeded))
        ));
        let empty = ChatIntent::Room {
            room: ChatRoom::World,
            text: String::new(),
        };
        assert!(matches!(
            session.chat(&empty).await,
            Err(SessionError::Chat(ChatWireError::Malformed))
        ));
        assert_eq!(session.next_command_id(), 7);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn chat_is_refused_without_capability_7_and_sends_nothing() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = script_peer(vec![]);
        let mut session = Session::admit(client, admission()?).await?;
        assert!(matches!(
            session.chat(&ChatIntent::OpenRoom(ChatRoom::World)).await,
            Err(SessionError::CapabilityNotSelected { capability: 7 })
        ));
        assert_eq!(session.next_command_id(), 7);
        assert!(matches!(
            session.step(StepDirection::East).await.err(),
            Some(SessionError::Io(_) | SessionError::Timeout { .. })
        ));
        drop(session);
        let _ = peer.await?;
        Ok(())
    })?
}
