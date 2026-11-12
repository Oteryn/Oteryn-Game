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
    // Soulseal fight command 6120 uses existing capability 72.
    // Coin market command 6128 uses existing capability 63.
    // Market details command 6126 uses existing capability 63.
    // Daily returner commands 6124 and 6125 use existing capability 182 (the picker 6122/6123 left it).
    // Store browse command 6118 uses existing capability 154.
    // Wheel presentation queries 6119 and 6121 use existing capability 148.
    // Chat console command 6117 uses existing capability 7; no new advertised capability.
    // PROTOCOL_OTERYN_V1_REGISTRY.json: capability 7 requires nothing.
    // Capability 272 adds live Atelier balances and retains its prerequisite 148.
    // pkg4 adds 92 (zones_nologout raid messages), 168 (animus mastery) and 274 (loyalty skills).
    // CL26 (#2816) adds the native client features 182 (daily reward), 224 (boss rewards, with
    // its prerequisite 117), 233 (player typing) and 259 (public channels, with its prerequisite
    // 80); the set stays strictly ascending as the hello encoder requires.
    // CL25J adds daily boost, forge, depot search, store, hotkey equip, browse field,
    // party chat and VIP presence, including the equipment/container prerequisites 12/14.
    // C78 adds 8 (live achievement notices).
    // C89 adds server-save countdown notices (75, domain 40).
    // P-C01 already advertises 67 (charm progression) and 87 (PvP expert).
    // P-C05 exposes the existing 117 routes (133, 169, 178-180, 340, 491); no table change.
    // P-C06 adds 39, 57 and 134; their prerequisites 4, 6 and 42 are already advertised.
    // P-C03 adds 97 (character info) and 149 (raid schedule query).
    // P-C04 adds 59 (house move-out), 86 (house transfer) and 99 (guildhall).
    // CP01 adds 54, 55, 115, 150 and the podium prerequisite 49.
    // C88 adds 53 (world-event query) through the admitted raw-command route.
    // C76 adds 9 (highscores), using the existing selected-session query route.
    // C10 adds counted item movement (61) and refusal details (66).
    // C12 adds item-look (28); world interaction (11) already carries bed support.
    // C04 adds 286 (live world entity display names); W14 adds 287 (spell catalogue).
    // P-C01 adds 67 (bestiary/charm progression) and 87 (PvP expert, domain 52).
    // C24 adds server target walking (64), with existing prerequisites 6 and 13.
    // C27 adds server weather (26, domain 22).
    // CLIGAP-W13 adds 83 (served World/Channel list, domain 48).
    // CLIGAP-W12 adds 10 (typed analyser facts and the native totals panel).
    // CLCAST adds native caster settings, chat and consent (290–292); Watch 289 is separate.
    // C17b adds 275 (empty-outfit-v1: invisible creatures project an empty outfit).
    // The native walking route also advertises diagonal movement (371).
    // The real-server client journeys (node boot) add 3 (npc_service) and 81 (npc_shop) for the
    // NPC shop phase, and 51 (staff) with 165 (hireling) for the hireling phase; staff negotiation
    // never grants the signed-envelope authority it requires.
    assert_eq!(
        CLIENT_SUPPORTED_CAPABILITIES,
        &[
            3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 28, 30,
            31, 36, 37, 39, 40, 42, 43, 49, 51, 53, 54, 55, 57, 59, 60, 61, 62, 63, 64, 66, 67, 68,
            72, 75, 80, 81, 82, 83, 86, 87, 91, 92, 97, 98, 99, 104, 115, 117, 132, 134, 140, 147,
            148, 149, 150, 151, 152, 153, 154, 165, 168, 179, 180, 182, 195, 199, 200, 207, 224,
            230, 232, 233, 240, 258, 259, 272, 274, 275, 276, 277, 278, 279, 280, 285, 286, 287,
            288, 290, 291, 292, 330, 350, 370, 371, 381, 382, 383, 384, 445, 446, 447, 448, 449,
            450, 461, 462, 463, 470, 474, 479, 480, 483, 485, 486, 487, 488, 489, 490, 491, 492,
            493, 494, 495, 496, 497, 498
        ]
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
fn local_clear_oldest_chat_line_keeps_owner_extras_and_future_lines_aligned() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), BoxError> {
            block_on(async {
                let (client, peer) = chat_peer(vec![], vec![]);
                let mut session = Session::admit(client, chat_admission()?).await?;
                let first = ChatLine::Private {
                    speaker_name: "Ari".into(),
                    text: "old".into(),
                };
                let second = ChatLine::Private {
                    speaker_name: "Ari".into(),
                    text: "kept".into(),
                };
                let extra = crate::chat_extended::LineExtra {
                    recipient_name: Some("Cy".into()),
                    line_sequence: Some(2),
                };
                let log = session.chat.as_mut().ok_or("capability 7 selected")?;
                log.push(
                    first,
                    crate::chat_extended::LineExtra {
                        recipient_name: Some("Bo".into()),
                        line_sequence: Some(1),
                    },
                );
                log.push(second.clone(), extra.clone());
                let revision = session.chat_revision;
                assert!(session.clear_oldest_chat_line());
                let log = session.chat_log().ok_or("capability 7 selected")?;
                assert_eq!(log.lines().cloned().collect::<Vec<_>>(), vec![second]);
                assert_eq!(log.line_extras().cloned().collect::<Vec<_>>(), vec![extra]);
                assert_eq!(session.chat_revision, revision);
                // Later incoming chat extends the same owner rather than restoring dismissed history.
                session.chat.as_mut().ok_or("chat owner")?.push(
                    room_line("incoming"),
                    crate::chat_extended::LineExtra::default(),
                );
                assert!(session.clear_oldest_chat_line());
                assert_eq!(
                    session
                        .chat_log()
                        .ok_or("chat owner")?
                        .lines()
                        .cloned()
                        .collect::<Vec<_>>(),
                    vec![room_line("incoming")]
                );
                assert!(session.clear_oldest_chat_line());
                assert!(!session.clear_oldest_chat_line());
                assert_eq!(
                    session.chat_log().ok_or("chat owner")?.line_extras().len(),
                    0
                );
                drop(session);
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "chat owner test thread panicked")?
}
