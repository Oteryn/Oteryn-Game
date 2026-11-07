//! Session consumers must enforce negotiation before applying the widened codec.
use super::*;
use crate::tests::{BoxError, admission_with, block_on, join_peer_with, spatial};
use oteryn_protocol_oteryn::{encode_command_result, encode_state_delta};
use tokio::io::DuplexStream;

const EXTRA_ROOMS: [ChatRoom; 4] = [
    ChatRoom::AdvertisingRookgaard,
    ChatRoom::Tutor,
    ChatRoom::Gamemaster,
    ChatRoom::GuildLeaders,
];

fn rooms(values: &[ChatRoom]) -> ChatRoomSet {
    let mut rooms = ChatRoomSet::default();
    for room in values {
        rooms.insert(*room);
    }
    rooms
}

fn line(room: ChatRoom) -> ChatLine {
    ChatLine::Room {
        room,
        speaker_name: "Moderator".into(),
        text: "channel message".into(),
    }
}

fn peer(
    selected: &'static [u32],
    snapshot_type: u32,
    snapshot: Vec<u8>,
    pushes: Vec<Vec<u8>>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(64 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            selected,
            selected,
            &[],
            (1, spatial(0)),
            &[(STATE_DOMAIN_CHAT, 3, snapshot_type, snapshot)],
        )
        .await?;
        for push in pushes {
            write_frame(&mut server, &push).await?;
        }
        let mut unexpected = Vec::new();
        server.read_to_end(&mut unexpected).await?;
        assert!(
            unexpected.is_empty(),
            "refused session must send no command"
        );
        Ok(())
    });
    (client, peer)
}

#[test]
fn public_channel_session_snapshots_require_the_negotiated_room_extension() -> Result<(), BoxError>
{
    block_on(async {
        for room in EXTRA_ROOMS {
            for selected in [&[7, 80][..], &[7, 36, 80][..]] {
                let open = rooms(&[ChatRoom::Help, room]);
                let (snapshot_type, payload) = if selected.contains(&36) {
                    (
                        guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1,
                        guild_room::encode_snapshot(&guild_room::GuildChatSnapshot {
                            rooms: open,
                            guild: GuildRoomState::Absent,
                        })
                        .map_err(|error| format!("{error:?}"))?,
                    )
                } else {
                    (chat::SNAPSHOT_TYPE_CHAT_V1, chat::encode_chat_rooms(open))
                };
                let (client, peer) = peer(selected, snapshot_type, payload, vec![]);
                assert!(
                    matches!(
                        Session::admit(client, admission_with(selected)?).await,
                        Err(SessionError::CapabilityNotSelected { capability: 259 })
                    ),
                    "unselected snapshot room {room:?}"
                );
                peer.await??;
            }
        }
        Ok(())
    })?
}

#[test]
fn public_channel_session_pushes_fail_before_mutation_and_poison_the_session()
-> Result<(), BoxError> {
    block_on(async {
        let initial = rooms(&[ChatRoom::Help]);
        let mut cases = Vec::new();
        for room in EXTRA_ROOMS {
            cases.push((
                &[7, 80][..],
                259,
                chat::DELTA_TYPE_CHAT_ROOMS_V1,
                chat::encode_chat_rooms(rooms(&[ChatRoom::Help, room])),
            ));
            cases.push((
                &[7, 80][..],
                259,
                chat::DELTA_TYPE_CHAT_LINE_V1,
                chat::encode_chat_line(&line(room)).map_err(|error| format!("{error:?}"))?,
            ));
        }
        cases.push((
            &[7][..],
            80,
            chat::DELTA_TYPE_CHAT_LINE_V1,
            chat::encode_chat_line(&ChatLine::PrivateChannel(
                chat::private_channel::PrivateChannelLine {
                    channel_id: 100,
                    speaker_name: "Owner".into(),
                    text: "private".into(),
                },
            ))
            .map_err(|error| format!("{error:?}"))?,
        ));
        for (selected, capability, delta_type, payload) in cases {
            let push = encode_state_delta(1, 41, STATE_DOMAIN_CHAT, 3, 4, delta_type, &payload)?;
            let (client, peer) = peer(
                selected,
                chat::SNAPSHOT_TYPE_CHAT_V1,
                chat::encode_chat_rooms(initial),
                vec![push],
            );
            let mut session = Session::admit(client, admission_with(selected)?).await?;
            let error = session
                .service_liveness(Duration::from_millis(200))
                .await
                .err()
                .ok_or("unselected channel push must fail")?;
            assert!(
                matches!(error, SessionError::CapabilityNotSelected { capability: actual } if actual == capability)
            );
            assert_eq!(session.chat_revision, 3);
            let log = session.chat_log().ok_or("chat log")?;
            assert_eq!(log.rooms(), initial);
            assert!(log.is_empty());
            assert!(session.take_events().is_empty());
            assert!(matches!(
                session.chat(&ChatIntent::OpenRoom(ChatRoom::Help)).await,
                Err(SessionError::SessionUnusable)
            ));
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}

#[test]
fn public_channel_session_blocks_unselected_sends_and_accepts_legacy_room_refusals()
-> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(&mut server, &[7], &[7], &[], (1, spatial(0)), &[]).await?;
            let frame = read_frame(&mut server).await?;
            let envelope = decode_wire_envelope(&frame)?;
            let command = envelope.client_command(1)?;
            assert_eq!(command.command_id, 7);
            assert_eq!(command.command_type, COMMAND_TYPE_CHAT_INTENT);
            assert_eq!(
                chat::decode_chat_intent(command.payload).map_err(|error| format!("{error:?}"))?,
                ChatIntent::OpenRoom(ChatRoom::Help)
            );
            let payload = chat::encode_chat_intent_result(&chat::ChatIntentResult {
                disposition: ChatDisposition::NotAllowed,
                wait_seconds: 0,
            })
            .map_err(|error| format!("{error:?}"))?;
            write_frame(
                &mut server,
                &encode_command_result(1, 41, 7, CommandStatus::Accepted, &payload)?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[7])?).await?;
        for room in EXTRA_ROOMS {
            for intent in [
                ChatIntent::OpenRoom(room),
                ChatIntent::CloseRoom(room),
                ChatIntent::Room {
                    room,
                    text: "not sent".into(),
                },
            ] {
                assert!(matches!(
                    session.chat(&intent).await,
                    Err(SessionError::CapabilityNotSelected { capability: 259 })
                ));
                assert_eq!(session.next_command_id(), 7);
            }
        }
        assert!(matches!(
            session
                .chat(&ChatIntent::PrivateChannel(
                    chat::private_channel::PrivateChannelIntent::Create
                ))
                .await,
            Err(SessionError::CapabilityNotSelected { capability: 80 })
        ));
        assert_eq!(session.next_command_id(), 7);
        let outcome = session.chat(&ChatIntent::OpenRoom(ChatRoom::Help)).await?;
        assert_eq!(outcome.disposition, ChatDisposition::NotAllowed);
        assert_eq!(outcome.status, CommandStatus::Accepted);
        assert_eq!(session.next_command_id(), 8);
        session.ensure_usable()?;
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn public_channel_session_selected_extension_accepts_snapshot_deltas_and_refusal()
-> Result<(), BoxError> {
    block_on(async {
        let initial = rooms(&[ChatRoom::Help, ChatRoom::Tutor]);
        let opened = rooms(&ChatRoom::ALL);
        let public = line(ChatRoom::GuildLeaders);
        let private = ChatLine::PrivateChannel(chat::private_channel::PrivateChannelLine {
            channel_id: 100,
            speaker_name: "Owner".into(),
            text: "private".into(),
        });
        let expected = [public.clone(), private.clone()];
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[7, 80, 259],
                &[7, 80, 259],
                &[],
                (1, spatial(0)),
                &[(
                    STATE_DOMAIN_CHAT,
                    3,
                    chat::SNAPSHOT_TYPE_CHAT_V1,
                    chat::encode_chat_rooms(initial),
                )],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_CHAT,
                    3,
                    4,
                    chat::DELTA_TYPE_CHAT_ROOMS_V1,
                    &chat::encode_chat_rooms(opened),
                )?,
            )
            .await?;
            for (offset, line) in [public, private].iter().enumerate() {
                let offset = u64::try_from(offset)?;
                let payload = chat::encode_chat_line(line).map_err(|error| format!("{error:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(
                        1,
                        42 + offset,
                        STATE_DOMAIN_CHAT,
                        4 + offset,
                        5 + offset,
                        chat::DELTA_TYPE_CHAT_LINE_V1,
                        &payload,
                    )?,
                )
                .await?;
            }
            let frame = read_frame(&mut server).await?;
            let envelope = decode_wire_envelope(&frame)?;
            let command = envelope.client_command(1)?;
            assert_eq!(
                chat::decode_chat_intent(command.payload).map_err(|error| format!("{error:?}"))?,
                ChatIntent::OpenRoom(ChatRoom::Gamemaster)
            );
            let payload = chat::encode_chat_intent_result(&chat::ChatIntentResult {
                disposition: ChatDisposition::NotAllowed,
                wait_seconds: 0,
            })
            .map_err(|error| format!("{error:?}"))?;
            write_frame(
                &mut server,
                &encode_command_result(
                    1,
                    44,
                    command.command_id,
                    CommandStatus::Accepted,
                    &payload,
                )?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[7, 80, 259])?).await?;
        assert_eq!(session.chat_log().ok_or("chat log")?.rooms(), initial);
        let result = session
            .chat(&ChatIntent::OpenRoom(ChatRoom::Gamemaster))
            .await?;
        assert_eq!(result.disposition, ChatDisposition::NotAllowed);
        assert_eq!(result.status, CommandStatus::Accepted);
        let log = session.chat_log().ok_or("chat log")?;
        assert_eq!(log.rooms(), opened);
        assert_eq!(log.lines().cloned().collect::<Vec<_>>(), expected);
        assert_eq!(session.chat_revision, 6);
        assert_eq!(session.take_events().len(), 3);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn public_and_private_chat_session_selection_and_refusal_guards_are_closed() {
    for selected in [&[259][..], &[7, 259], &[80, 259], &[80]] {
        assert!(check_selection(selected).is_err());
    }
    assert!(check_selection(&[7, 80, 259]).is_ok());
    assert!(matches!(
        check_result(ChatDisposition::NotInvited, &[7]),
        Err(SessionError::CapabilityNotSelected { capability: 80 })
    ));
    assert!(check_result(ChatDisposition::NotInvited, &[7, 80]).is_ok());
    assert!(check_result(ChatDisposition::NotAllowed, &[7, 80]).is_ok());
    assert!(
        check_result_for_intent(
            ChatDisposition::NotAllowed,
            &[7],
            &ChatIntent::OpenRoom(ChatRoom::Help)
        )
        .is_ok()
    );
    assert!(
        check_result_for_intent(
            ChatDisposition::NotAllowed,
            &[7],
            &ChatIntent::OpenRoom(ChatRoom::Tutor)
        )
        .is_err()
    );
}
