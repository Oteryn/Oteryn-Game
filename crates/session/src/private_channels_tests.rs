use crate::tests::{BoxError, admission_with, block_on, join_peer_with, spatial};
use crate::*;
use oteryn_protocol_oteryn::chat::private_channel as wire;
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use tokio::io::DuplexStream;

const PRIVATE: &[u32] = &[7, 80];
const PUBLIC_PRIVATE: &[u32] = &[7, 80, 259];

fn rooms(values: &[ChatRoom]) -> ChatRoomSet {
    let mut rooms = ChatRoomSet::default();
    for room in values {
        rooms.insert(*room);
    }
    rooms
}

fn list() -> ChatChannelList {
    ChatChannelList {
        public_rooms: rooms(&[ChatRoom::World, ChatRoom::Help]),
        own_channel: Some(100),
        invited: vec![InvitedChannel {
            channel_id: 101,
            owner_name: "Other Player".into(),
        }],
    }
}

async fn admitted(
    selected: &'static [u32],
) -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut server) = tokio::io::duplex(1 << 16);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            selected,
            selected,
            &[],
            (1, spatial(0)),
            &[(
                STATE_DOMAIN_CHAT,
                3,
                chat::SNAPSHOT_TYPE_CHAT_V1,
                chat::encode_chat_rooms(rooms(&[ChatRoom::English])),
            )],
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let session = Session::admit(client, admission_with(selected)?).await?;
    Ok((session, peer.await??))
}

fn list_frame(
    sequence: u64,
    base: u64,
    new: u64,
    value: &ChatChannelList,
) -> Result<Vec<u8>, BoxError> {
    let payload = wire::encode_channel_list(value).map_err(|error| format!("{error:?}"))?;
    Ok(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHAT,
        base,
        new,
        wire::DELTA_TYPE_CHAT_CHANNEL_LIST_V1,
        &payload,
    )?)
}

fn apply(
    session: &mut Session<DuplexStream>,
    base: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let frame = encode_state_delta(
        1,
        41,
        STATE_DOMAIN_CHAT,
        base,
        base + 1,
        wire::DELTA_TYPE_CHAT_CHANNEL_LIST_V1,
        payload,
    )?;
    session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())
}

#[test]
fn private_channel_list_client_applies_replacements_as_chat_events() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, mut server) = admitted(PRIVATE).await?;
        assert!(session.private_channel_list().is_none());
        let first = list();
        write_frame(&mut server, &list_frame(41, 3, 4, &first)?).await?;
        session.service_liveness(Duration::from_millis(50)).await?;
        assert_eq!(session.private_channel_list(), Some(&first));
        let events = session.take_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], SessionEvent::ChatChannelList(delta)
            if delta.server_sequence == 41 && delta.base_revision == 3
                && delta.new_revision == 4 && delta.value == first));
        assert_eq!(events[0].domain_id(), STATE_DOMAIN_CHAT);
        let log = session.chat_log().ok_or("chat log")?;
        assert_eq!(log.private_channels(), Some(&first));
        assert_eq!(
            log.rooms(),
            rooms(&[ChatRoom::English]),
            "available rooms do not open themselves"
        );
        assert!(log.is_empty(), "channel metadata is not a chat line");

        let line = ChatLine::PrivateChannel(PrivateChannelLine {
            channel_id: 100,
            speaker_name: "Owner".into(),
            text: "hello".into(),
        });
        let payload = chat::encode_chat_line(&line).map_err(|error| format!("{error:?}"))?;
        write_frame(
            &mut server,
            &encode_state_delta(
                1,
                42,
                STATE_DOMAIN_CHAT,
                4,
                5,
                chat::DELTA_TYPE_CHAT_LINE_V1,
                &payload,
            )?,
        )
        .await?;
        let empty = ChatChannelList::default();
        write_frame(&mut server, &list_frame(43, 5, 6, &empty)?).await?;
        session.service_liveness(Duration::from_millis(50)).await?;
        let events = session.take_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], SessionEvent::ChatLine(delta) if delta.value == line));
        assert!(matches!(&events[1], SessionEvent::ChatChannelList(delta)
            if delta.server_sequence == 43 && delta.base_revision == 5 && delta.new_revision == 6
                && delta.value == empty));
        assert_eq!(
            session.private_channel_list(),
            Some(&empty),
            "whole replacement removes old invitations"
        );
        assert_eq!(
            session.chat_revision, 6,
            "line and list share one CHAT revision"
        );
        assert_eq!(
            session.chat_log().ok_or("chat log")?.lines().last(),
            Some(&line)
        );
        Ok(())
    })?
}

#[test]
fn private_channel_list_client_requires_cap80_before_any_state_change() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, _server) = admitted(&[7]).await?;
        let before = session.chat_log().cloned();
        let payload = wire::encode_channel_list(&list()).map_err(|error| format!("{error:?}"))?;
        assert!(matches!(
            apply(&mut session, 3, &payload),
            Err(SessionError::CapabilityNotSelected { capability: 80 })
        ));
        assert_eq!(session.chat_log(), before.as_ref());
        assert!(session.private_channel_list().is_none());
        assert_eq!(session.chat_revision, 3);
        assert!(session.take_events().is_empty());
        Ok(())
    })?
}

#[test]
fn private_channel_list_client_refuses_malformed_overflow_and_stale_revisions_atomically()
-> Result<(), BoxError> {
    block_on(async {
        let (mut session, _server) = admitted(PRIVATE).await?;
        let mut full = list();
        full.invited = (101..117)
            .map(|channel_id| InvitedChannel {
                channel_id,
                owner_name: "Other Player".into(),
            })
            .collect();
        let payload = wire::encode_channel_list(&full).map_err(|error| format!("{error:?}"))?;
        apply(&mut session, 3, &payload)?;
        assert_eq!(
            session.private_channel_list().ok_or("list")?.invited.len(),
            16
        );
        let before = session.chat_log().cloned();
        let mut overflow = payload.clone();
        overflow.extend_from_slice(&[0x1a, 0x05, 0x08, 117, 0x12, 0x01, b'X']);
        for malformed in [
            vec![0x10, 0],              // zero own channel
            vec![0x10, 100, 0x10, 101], // duplicate own field
            vec![0; wire::MAX_CHAT_CHANNEL_LIST_BYTES + 1],
            overflow, // seventeenth invitation
        ] {
            assert!(matches!(
                apply(&mut session, 4, &malformed),
                Err(SessionError::Chat(_))
            ));
            assert_eq!(session.chat_log(), before.as_ref());
            assert_eq!(session.chat_revision, 4);
        }
        assert!(matches!(
            apply(&mut session, 3, &payload),
            Err(SessionError::StateRevisionMismatch {
                expected_base: 4,
                actual_base: 3,
                ..
            })
        ));
        assert_eq!(session.chat_log(), before.as_ref());
        assert_eq!(session.chat_revision, 4);
        assert!(session.take_events().is_empty());
        Ok(())
    })?
}

#[test]
fn private_channel_list_client_requires_cap259_for_added_public_rooms() -> Result<(), BoxError> {
    block_on(async {
        let mut value = list();
        value.public_rooms.insert(ChatRoom::Tutor);
        let payload = wire::encode_channel_list(&value).map_err(|error| format!("{error:?}"))?;
        let (mut legacy, _legacy_server) = admitted(PRIVATE).await?;
        let before = legacy.chat_log().cloned();
        assert!(matches!(
            apply(&mut legacy, 3, &payload),
            Err(SessionError::CapabilityNotSelected { capability: 259 })
        ));
        assert_eq!(legacy.chat_log(), before.as_ref());
        assert_eq!(legacy.chat_revision, 3);
        let (mut selected, _server) = admitted(PUBLIC_PRIVATE).await?;
        assert!(matches!(
            apply(&mut selected, 3, &payload),
            Ok(SessionEvent::ChatChannelList(_))
        ));
        assert_eq!(selected.private_channel_list(), Some(&value));
        assert_eq!(selected.chat_revision, 4);
        Ok(())
    })?
}
