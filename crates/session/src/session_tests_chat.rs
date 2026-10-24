// --- CHAT-CLIENT-1: capability 7, command 13 and domain 12.

fn room_set(rooms: &[ChatRoom]) -> ChatRoomSet {
    let mut set = ChatRoomSet::default();
    for room in rooms {
        set.insert(*room);
    }
    set
}

fn chat_admission() -> Result<Admission<'static>, BoxError> {
    admission_with(&[CAPABILITY_CHAT_V1, 13])
}

fn chat_domain(revision: u64, rooms: ChatRoomSet) -> (u32, u64, u32, Vec<u8>) {
    (
        STATE_DOMAIN_CHAT,
        revision,
        chat::SNAPSHOT_TYPE_CHAT_V1,
        chat::encode_chat_rooms(rooms),
    )
}

/// Joins with capability 7 selected and `raw` as extra domains, then plays `script`.
fn chat_peer(
    raw: Vec<(u32, u64, u32, Vec<u8>)>,
    script: Vec<Step>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            &[CAPABILITY_CHAT_V1, 13],
            &[CAPABILITY_CHAT_V1, 13],
            &[],
            (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
            &raw,
        )
        .await?;
        for step in script {
            play_step(&mut server, step).await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    (client, peer)
}

fn chat_line_push(sequence: u64, base: u64, new: u64, line: &ChatLine) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHAT,
        base,
        new,
        chat::DELTA_TYPE_CHAT_LINE_V1,
        &chat::encode_chat_line(line).map_err(|error| format!("{error:?}"))?,
    )?))
}

fn chat_rooms_push(
    sequence: u64,
    base: u64,
    new: u64,
    rooms: ChatRoomSet,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHAT,
        base,
        new,
        chat::DELTA_TYPE_CHAT_ROOMS_V1,
        &chat::encode_chat_rooms(rooms),
    )?))
}

fn chat_result(
    sequence: u64,
    id: u64,
    status: CommandStatus,
    disposition: ChatDisposition,
    wait_seconds: u32,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_command_result(
        1,
        sequence,
        id,
        status,
        &chat::encode_chat_intent_result(&chat::ChatIntentResult {
            disposition,
            wait_seconds,
        })
        .map_err(|error| format!("{error:?}"))?,
    )?))
}

fn room_line(text: &str) -> ChatLine {
    ChatLine::Room {
        room: ChatRoom::Help,
        speaker_name: "Ada".to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn the_advertised_set_carries_chat_and_stays_closed() {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&CAPABILITY_CHAT_V1));
    // PROTOCOL_OTERYN_V1_REGISTRY.json: capability 7 requires nothing.
    assert_eq!(
        CLIENT_SUPPORTED_CAPABILITIES,
        &[4, 6, 7, 13, 17, 18, 21, 25, 31, 36, 37]
    );
}

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

#[test]
fn muted_and_exhausted_results_report_their_wait_and_change_nothing() -> Result<(), BoxError> {
    block_on(async {
        let open = room_set(&[ChatRoom::World]);
        let (client, peer) = chat_peer(
            vec![chat_domain(3, open)],
            vec![
                Step::ReadCommand,
                chat_result(41, 7, CommandStatus::Accepted, ChatDisposition::Muted, 30)?,
                Step::ReadCommand,
                chat_result(
                    42,
                    8,
                    CommandStatus::Accepted,
                    ChatDisposition::Exhausted,
                    1_280,
                )?,
            ],
        );
        let mut session = Session::admit(client, chat_admission()?).await?;
        let say = ChatIntent::Say {
            mode: ChatSpeechMode::Yell,
            text: "hello".to_owned(),
        };
        let muted = session.chat(&say).await?;
        assert_eq!(
            (muted.disposition, muted.wait_seconds),
            (ChatDisposition::Muted, 30)
        );
        let exhausted = session.chat(&say).await?;
        assert_eq!(
            (exhausted.disposition, exhausted.wait_seconds),
            (ChatDisposition::Exhausted, 1_280)
        );
        assert!(session.take_events().is_empty());
        let log = session.chat_log().ok_or("capability 7 selected")?;
        assert!((log.rooms(), log.len()) == (open, 0));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_status_that_contradicts_the_disposition_fails_closed() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = chat_peer(
            vec![],
            vec![
                Step::ReadCommand,
                chat_result(41, 7, CommandStatus::Accepted, ChatDisposition::Rejected, 0)?,
            ],
        );
        let mut session = Session::admit(client, chat_admission()?).await?;
        assert!(matches!(
            session.chat(&ChatIntent::OpenRoom(ChatRoom::Help)).await,
            Err(SessionError::InconsistentCommandResult { command_id: 7, .. })
        ));
        assert!(matches!(
            session.chat(&ChatIntent::OpenRoom(ChatRoom::Help)).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_line_pushed_by_another_speaker_applies_while_idle() -> Result<(), BoxError> {
    block_on(async {
        let local = ChatLine::Local {
            speaker: ChatSpeaker {
                identity: [9; 16],
                generation: std::num::NonZeroU64::MIN,
            },
            speaker_name: "Bob".to_owned(),
            mode: ChatSpeechMode::Yell,
            text: "over here".to_owned(),
            position: at(2, 1),
        };
        let private = ChatLine::Private {
            speaker_name: "Cy".to_owned(),
            text: "psst".to_owned(),
        };
        let lines = [local, private, room_line("anyone?"), ChatLine::Dropped];
        let mut script = Vec::new();
        for (offset, line) in (0_u64..).zip(&lines) {
            script.push(chat_line_push(41 + offset, 3 + offset, 4 + offset, line)?);
        }
        let (client, peer) = chat_peer(vec![chat_domain(3, ChatRoomSet::default())], script);
        let mut session = Session::admit(client, chat_admission()?).await?;
        session.service_liveness(Duration::from_millis(300)).await?;
        let events = session.take_events();
        assert_eq!(events.len(), 4);
        for ((offset, event), line) in (0_u64..).zip(&events).zip(&lines) {
            assert!(matches!(
                event,
                SessionEvent::ChatLine(applied)
                    if applied.base_revision == 3 + offset
                        && applied.new_revision == 4 + offset
                        && applied.value == *line
            ));
        }
        let log = session.chat_log().ok_or("capability 7 selected")?;
        assert_eq!(log.lines().cloned().collect::<Vec<_>>(), lines);
        assert_eq!(events[3].domain_id(), STATE_DOMAIN_CHAT);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_rooms_delta_replaces_the_open_set_and_lines_keep_the_last_64() -> Result<(), BoxError> {
    block_on(async {
        let closed = room_set(&[ChatRoom::English]);
        let mut script = vec![chat_rooms_push(41, 3, 4, closed)?];
        for index in 0..=MAX_CHAT_LOG_LINES {
            let offset = u64::try_from(index)?;
            script.push(chat_line_push(
                42 + offset,
                4 + offset,
                5 + offset,
                &room_line(&format!("line {index}")),
            )?);
        }
        let (client, peer) = chat_peer(
            vec![chat_domain(3, room_set(&[ChatRoom::World, ChatRoom::Help]))],
            script,
        );
        let mut session = Session::admit(client, chat_admission()?).await?;
        session.service_liveness(Duration::from_millis(500)).await?;
        let events = session.take_events();
        assert_eq!(events.len(), MAX_CHAT_LOG_LINES + 2);
        assert!(matches!(
            events.first(),
            Some(SessionEvent::ChatRooms(applied)) if applied.value == closed
        ));
        let log = session.chat_log().ok_or("capability 7 selected")?;
        assert_eq!(log.rooms(), closed);
        assert_eq!(log.len(), MAX_CHAT_LOG_LINES);
        // 65 lines were pushed: the oldest ("line 0") fell out of the ring.
        assert_eq!(log.lines().next(), Some(&room_line("line 1")));
        assert_eq!(
            log.lines().last(),
            Some(&room_line(&format!("line {MAX_CHAT_LOG_LINES}")))
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_contradicting_chat_delta_poisons_the_session() -> Result<(), BoxError> {
    block_on(async {
        for (push, check) in [
            (
                // Base revision mismatch.
                chat_line_push(41, 2, 3, &room_line("x"))?,
                (|error: &SessionError| {
                    matches!(
                        error,
                        SessionError::StateRevisionMismatch {
                            domain_id: STATE_DOMAIN_CHAT,
                            expected_base: 3,
                            actual_base: 2
                        }
                    )
                }) as fn(&SessionError) -> bool,
            ),
            (
                // An unregistered delta type.
                Step::Send(encode_state_delta(1, 41, STATE_DOMAIN_CHAT, 3, 4, 3, b"x")?),
                |error| {
                    matches!(
                        error,
                        SessionError::UnregisteredDeltaType {
                            domain_id: STATE_DOMAIN_CHAT,
                            delta_type: 3
                        }
                    )
                },
            ),
            (
                // A payload the codec refuses.
                Step::Send(encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_CHAT,
                    3,
                    4,
                    chat::DELTA_TYPE_CHAT_LINE_V1,
                    b"\xff",
                )?),
                |error| matches!(error, SessionError::Chat(_)),
            ),
        ] {
            let (client, peer) =
                chat_peer(vec![chat_domain(3, ChatRoomSet::default())], vec![push]);
            let mut session = Session::admit(client, chat_admission()?).await?;
            let error = session
                .service_liveness(Duration::from_millis(200))
                .await
                .err()
                .ok_or("the push must fail closed")?;
            assert!(check(&error), "{error:?}");
            assert!(session.chat_log().is_some_and(ChatLog::is_empty));
            assert!(matches!(
                session.chat(&ChatIntent::OpenRoom(ChatRoom::World)).await,
                Err(SessionError::SessionUnusable)
            ));
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}
