#![allow(clippy::expect_used)]
use super::*;

fn name(bytes: usize) -> String {
    "N".repeat(bytes)
}

fn largest_text() -> String {
    "\u{10FFFF}".repeat(MAX_CHAT_TEXT_BYTES / 4)
}

#[test]
fn the_private_echo_is_kind_two_with_field_twelve_after_the_legacy_bytes() {
    let echo = PrivateEcho {
        speaker_name: "Aldric".into(),
        recipient_name: "Bryn".into(),
        text: "hi".into(),
    };
    let bytes = encode_private_echo(&echo).expect("valid fixture");
    let legacy = encode_chat_line(&ChatLine::Private {
        speaker_name: "Aldric".into(),
        text: "hi".into(),
    })
    .expect("valid fixture");
    assert_eq!(
        bytes,
        [legacy.clone(), vec![98, 4, b'B', b'r', b'y', b'n']].concat()
    );
    assert_eq!(
        decode_chat_line_extended(&bytes).expect("valid fixture"),
        ExtendedChatLine {
            body: ChatLineBody::PrivateEcho(echo),
            line_sequence: None,
        }
    );
    // The legacy decoder never accepts field 12, so an unselected session is never sent it.
    assert!(decode_chat_line(&bytes).is_err());
    // Field 12 belongs to kind 2 only, at most once, and is a valid name.
    let room = encode_chat_line(&ChatLine::Room {
        room: ChatRoom::World,
        speaker_name: "Aldric".into(),
        text: "hi".into(),
    })
    .expect("valid fixture");
    for payload in [
        [room, vec![98, 1, b'B']].concat(),
        [legacy.clone(), vec![98, 1, b'B', 98, 1, b'C']].concat(),
        [legacy.clone(), vec![98, 0]].concat(),
        [legacy, vec![98, 1, 0xff]].concat(),
    ] {
        assert!(decode_chat_line_extended(&payload).is_err(), "{payload:?}");
    }
    assert!(
        encode_private_echo(&PrivateEcho {
            speaker_name: "Aldric".into(),
            recipient_name: name(MAX_CHAT_NAME_BYTES + 1),
            text: "hi".into(),
        })
        .is_err()
    );
}

#[test]
fn the_largest_extended_line_is_the_bound() {
    let echo = PrivateEcho {
        speaker_name: name(MAX_CHAT_NAME_BYTES),
        recipient_name: name(MAX_CHAT_NAME_BYTES),
        text: largest_text(),
    };
    let mut bytes = encode_private_echo(&echo).expect("valid fixture");
    append_line_sequence(&mut bytes, u64::MAX).expect("valid fixture");
    assert_eq!(bytes.len(), MAX_CHAT_LINE_EXTENDED_BYTES);
    assert_eq!(
        decode_chat_line_extended(&bytes).expect("valid fixture"),
        ExtendedChatLine {
            body: ChatLineBody::PrivateEcho(echo),
            line_sequence: Some(u64::MAX),
        }
    );
    bytes.push(0);
    assert_eq!(
        decode_chat_line_extended(&bytes),
        Err(ChatWireError::LimitExceeded)
    );
    // Every legacy line with a sequence stays within the bound.
    let mut local = encode_chat_line(&ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [7; ENTITY_IDENTITY_BYTES],
            generation: NonZeroU64::MAX,
        },
        speaker_name: name(MAX_CHAT_NAME_BYTES),
        mode: ChatSpeechMode::Yell,
        text: largest_text(),
        position: ActorPosition {
            x: i32::MAX,
            y: i32::MAX,
            floor: 15,
        },
    })
    .expect("valid fixture");
    append_line_sequence(&mut local, u64::MAX).expect("valid fixture");
    assert!(local.len() <= MAX_CHAT_LINE_EXTENDED_BYTES);
}

#[test]
fn the_channel_notice_is_line_kind_eight_round_trip() {
    for kind in [
        ChannelNoticeKind::Joined,
        ChannelNoticeKind::Left,
        ChannelNoticeKind::Excluded,
    ] {
        let notice = ChannelNotice {
            channel_id: 9_999,
            kind,
            name: name(MAX_CHAT_NAME_BYTES),
        };
        let mut bytes = encode_channel_notice(&notice).expect("valid fixture");
        append_line_sequence(&mut bytes, 3).expect("valid fixture");
        assert_eq!(
            decode_chat_line_extended(&bytes).expect("valid fixture"),
            ExtendedChatLine {
                body: ChatLineBody::ChannelNotice(notice),
                line_sequence: Some(3),
            }
        );
        assert!(decode_chat_line(&bytes).is_err());
    }
    let bytes = encode_channel_notice(&ChannelNotice {
        channel_id: 100,
        kind: ChannelNoticeKind::Left,
        name: "Bryn".into(),
    })
    .expect("valid fixture");
    assert_eq!(
        bytes,
        [8, 8, 34, 4, b'B', b'r', b'y', b'n', 88, 100, 112, 2]
    );
    // No kind, kind 0 or 4, channel 0, a missing name, text or a recipient all fail closed.
    for payload in [
        vec![8, 8, 34, 1, b'B', 88, 100],
        vec![8, 8, 34, 1, b'B', 88, 100, 112, 0],
        vec![8, 8, 34, 1, b'B', 88, 100, 112, 4],
        vec![8, 8, 34, 1, b'B', 88, 0, 112, 1],
        vec![8, 8, 88, 100, 112, 1],
        vec![8, 8, 34, 1, b'B', 58, 1, b'x', 88, 100, 112, 1],
        vec![8, 8, 34, 1, b'B', 88, 100, 112, 1, 98, 1, b'C'],
    ] {
        assert!(decode_chat_line_extended(&payload).is_err(), "{payload:?}");
    }
    // Field 14 belongs to kind 8 only.
    assert!(decode_chat_line_extended(&[8, 4, 112, 1]).is_err());
    assert!(
        encode_channel_notice(&ChannelNotice {
            channel_id: 0,
            kind: ChannelNoticeKind::Joined,
            name: "Bryn".into(),
        })
        .is_err()
    );
}

#[test]
fn the_line_sequence_is_field_thirteen_on_any_line() {
    let mut bytes = encode_chat_line(&ChatLine::Dropped).expect("valid fixture");
    assert_eq!(bytes, [8, 4]);
    append_line_sequence(&mut bytes, 300).expect("valid fixture");
    assert_eq!(bytes, [8, 4, 104, 172, 2]);
    assert_eq!(
        decode_chat_line_extended(&bytes).expect("valid fixture"),
        ExtendedChatLine {
            body: ChatLineBody::Line(ChatLine::Dropped),
            line_sequence: Some(300),
        }
    );
    assert!(decode_chat_line(&bytes).is_err());
    assert!(append_line_sequence(&mut bytes, 0).is_err());
    // Sequence 0 or a repeated sequence fail closed; a legacy line decodes unchanged.
    for payload in [vec![8, 4, 104, 0], vec![8, 4, 104, 1, 104, 2]] {
        assert!(decode_chat_line_extended(&payload).is_err(), "{payload:?}");
    }
    let party = ChatLine::Party {
        speaker_name: "Aldric".into(),
        text: "hi".into(),
    };
    let legacy = encode_chat_line(&party).expect("valid fixture");
    assert_eq!(
        decode_chat_line_extended(&legacy).expect("valid fixture"),
        ExtendedChatLine {
            body: ChatLineBody::Line(party),
            line_sequence: None,
        }
    );
}
