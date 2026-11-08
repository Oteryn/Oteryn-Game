//! S04b replay: the private-channel invitation list and a channel line applied through
//! `Session::apply_state_delta` (the capability-80 decode in `private_channels.rs`) and exposed by
//! `Session::private_channel_list` and `Session::chat_log`, the values `DevClientSession` and the
//! channel panel read.
use super::*;
use oteryn_protocol_oteryn::chat::private_channel as wire;
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use tokio::io::DuplexStream;

const SELECTED: &[u32] = &[7, 80, 259];

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("s04b-invited-list.bin"))?,
            std::fs::read(root.join("s04b-line.bin"))?,
            std::fs::read_to_string(root.join("s04b-expected.txt"))?,
        ));
    }
    let list = wire::encode_channel_list(&ChatChannelList {
        public_rooms: ChatRoomSet::default(),
        own_channel: None,
        invited: vec![InvitedChannel {
            channel_id: 101,
            owner_name: "Aurelia".into(),
        }],
    })
    .map_err(|error| format!("{error:?}"))?;
    let line = chat::encode_chat_line(&ChatLine::PrivateChannel(PrivateChannelLine {
        channel_id: 101,
        speaker_name: "Aurelia".into(),
        text: "s04b-channel-line".into(),
    }))
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        list,
        line,
        "owner=Aurelia\nchannel=101\ntext=s04b-channel-line\nselected=7,80,259\nrefusal=NotInvited\n".into(),
    ))
}

async fn admitted() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut server) = tokio::io::duplex(1 << 16);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            SELECTED,
            SELECTED,
            &[],
            (1, spatial(0)),
            &[(
                STATE_DOMAIN_CHAT,
                3,
                chat::SNAPSHOT_TYPE_CHAT_V1,
                chat::encode_chat_rooms(ChatRoomSet::default()),
            )],
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let session = Session::admit(client, admission_with(SELECTED)?).await?;
    Ok((session, peer.await??))
}

fn delta(
    session: &mut Session<DuplexStream>,
    sequence: u64,
    base: u64,
    delta_type: u32,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let frame = encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHAT,
        base,
        base + 1,
        delta_type,
        payload,
    )?;
    session.apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())
}

#[test]
fn e2e_s04b_private_channel_invitation_and_line_replay() -> Result<(), BoxError> {
    let (list_payload, line_payload, expected) = load()?;
    let owner = value(&expected, "owner")?;
    let channel: u16 = value(&expected, "channel")?.parse()?;
    let text = value(&expected, "text")?;
    block_on(async {
        let (mut session, _server) = admitted().await?;
        assert!(session.private_channel_list().is_none());

        let event = delta(
            &mut session,
            41,
            3,
            wire::DELTA_TYPE_CHAT_CHANNEL_LIST_V1,
            &list_payload,
        )?;
        let SessionEvent::ChatChannelList(applied) = event else {
            return Err("list delta did not produce a channel-list event".into());
        };
        assert_eq!((applied.base_revision, applied.new_revision), (3, 4));
        assert_eq!(applied.value.own_channel, None);
        assert_eq!(
            applied.value.invited,
            vec![InvitedChannel {
                channel_id: channel,
                owner_name: owner.into(),
            }]
        );
        assert_eq!(session.private_channel_list(), Some(&applied.value));

        let event = delta(
            &mut session,
            42,
            4,
            chat::DELTA_TYPE_CHAT_LINE_V1,
            &line_payload,
        )?;
        let expected_line = ChatLine::PrivateChannel(PrivateChannelLine {
            channel_id: channel,
            speaker_name: owner.into(),
            text: text.into(),
        });
        assert!(
            matches!(&event, SessionEvent::ChatLine(applied) if applied.value == expected_line)
        );
        assert_eq!(
            session.chat_log().ok_or("chat log absent")?.lines().last(),
            Some(&expected_line)
        );

        // A truncated list is refused whole: the list, the log and the revision stay as applied.
        let before_list = session.private_channel_list().cloned();
        let before_revision = session.chat_revision;
        let truncated = &list_payload[..list_payload.len() - 1];
        assert!(
            delta(
                &mut session,
                43,
                5,
                wire::DELTA_TYPE_CHAT_CHANNEL_LIST_V1,
                truncated
            )
            .is_err()
        );
        assert_eq!(session.private_channel_list().cloned(), before_list);
        assert_eq!(session.chat_revision, before_revision);
        Ok(())
    })?
}
