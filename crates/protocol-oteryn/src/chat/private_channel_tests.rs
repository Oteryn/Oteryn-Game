#![allow(clippy::expect_used)]
use super::*;

fn roundtrip(intent: PrivateChannelIntent) {
    let intent = ChatIntent::PrivateChannel(intent);
    let bytes = encode_chat_intent(&intent).expect("valid fixture");
    assert!(bytes.len() <= MAX_CHAT_INTENT_BYTES);
    assert_eq!(decode_chat_intent(&bytes).expect("valid fixture"), intent);
}

#[test]
fn private_channel_intents_roundtrip_within_the_chat_intent_bound() {
    roundtrip(PrivateChannelIntent::Create);
    roundtrip(PrivateChannelIntent::List);
    roundtrip(PrivateChannelIntent::Invite {
        name: "N".repeat(MAX_CHAT_NAME_BYTES),
    });
    roundtrip(PrivateChannelIntent::Exclude {
        name: "Bryn".into(),
    });
    roundtrip(PrivateChannelIntent::Join { channel_id: 1 });
    roundtrip(PrivateChannelIntent::Leave {
        channel_id: u16::MAX,
    });
    roundtrip(PrivateChannelIntent::Say {
        channel_id: u16::MAX,
        text: "\u{10FFFF}".repeat(MAX_CHAT_TEXT_BYTES / 4),
    });
}

#[test]
fn private_channel_intents_are_strict() {
    // Channel 0, a channel beyond u16, a missing channel or text, an alien field, a name on a
    // join and text on a create all fail closed, as does an empty or oversized name.
    for payload in [
        vec![82, 2, 8, 0],
        vec![82, 4, 8, 0x80, 0x80, 0x04],
        vec![82, 0],
        vec![98, 2, 8, 1],
        vec![98, 4, 18, 2, b'h', b'i'],
        vec![82, 3, 10, 1, b'A'],
        vec![58, 2, 8, 1],
        vec![66, 0],
        vec![66, 2, 10, 0],
        vec![106, 2, 8, 1],
        vec![114, 0],
    ] {
        assert!(decode_chat_intent(&payload).is_err(), "{payload:?}");
    }
    assert!(
        encode_chat_intent(&ChatIntent::PrivateChannel(PrivateChannelIntent::Join {
            channel_id: 0
        }))
        .is_err()
    );
    assert!(
        encode_chat_intent(&ChatIntent::PrivateChannel(PrivateChannelIntent::Invite {
            name: "N".repeat(MAX_CHAT_NAME_BYTES + 1)
        }))
        .is_err()
    );
}

#[test]
fn a_private_channel_line_is_line_kind_six_within_the_line_bound() {
    let line = ChatLine::PrivateChannel(PrivateChannelLine {
        channel_id: u16::MAX,
        speaker_name: "N".repeat(MAX_CHAT_NAME_BYTES),
        text: "\u{10FFFF}".repeat(MAX_CHAT_TEXT_BYTES / 4),
    });
    let bytes = encode_chat_line(&line).expect("valid fixture");
    assert!(bytes.len() <= MAX_CHAT_LINE_BYTES);
    assert_eq!(decode_chat_line(&bytes).expect("valid fixture"), line);
    // Field 11 belongs to kind 6 only; kind 6 takes no room, no position and no channel 0.
    let room_with_channel = [vec![8, 3, 34, 1, b'A', 48, 1, 58, 1, b'x'], vec![88, 1]].concat();
    assert!(decode_chat_line(&room_with_channel).is_err());
    for forged in [vec![48, 1], vec![88, 0], vec![88, 1, 88, 1]] {
        let payload = [vec![8, 6, 34, 1, b'A', 58, 1, b'x'], forged].concat();
        assert!(decode_chat_line(&payload).is_err(), "{payload:?}");
    }
    assert!(decode_chat_line(&[8, 6, 34, 1, b'A', 58, 1, b'x']).is_err());
}

#[test]
fn the_channel_list_is_bounded_and_strict() {
    let mut rooms = ChatRoomSet::default();
    for room in ChatRoom::ALL {
        rooms.insert(room);
    }
    let full = ChatChannelList {
        public_rooms: rooms,
        own_channel: Some(u16::MAX),
        invited: (1..=MAX_INVITED_PRIVATE_CHANNELS)
            .map(|index| InvitedChannel {
                channel_id: u16::MAX - u16::try_from(index).expect("bounded"),
                owner_name: "N".repeat(MAX_CHAT_NAME_BYTES),
            })
            .collect(),
    };
    let bytes = encode_channel_list(&full).expect("valid fixture");
    assert_eq!(bytes.len(), MAX_CHAT_CHANNEL_LIST_BYTES);
    assert_eq!(decode_channel_list(&bytes).expect("valid fixture"), full);
    let empty = ChatChannelList::default();
    assert_eq!(
        decode_channel_list(&encode_channel_list(&empty).expect("valid fixture"))
            .expect("valid fixture"),
        empty
    );

    let mut over = full.clone();
    over.invited.push(InvitedChannel {
        channel_id: 1,
        owner_name: "A".into(),
    });
    assert_eq!(
        encode_channel_list(&over),
        Err(ChatWireError::LimitExceeded)
    );
    let mut duplicate = full.clone();
    duplicate.invited[1].channel_id = duplicate.invited[0].channel_id;
    assert!(encode_channel_list(&duplicate).is_err());
    let mut own_invited = full;
    own_invited.invited[0].channel_id = u16::MAX;
    assert!(encode_channel_list(&own_invited).is_err());
    // Room bit 4, a zero room set, channel 0 and an alien field fail closed.
    for payload in [
        vec![8, 16],
        vec![8, 0],
        vec![16, 0],
        vec![26, 2, 8, 1],
        vec![32, 1],
    ] {
        assert!(decode_channel_list(&payload).is_err(), "{payload:?}");
    }
}

#[test]
fn not_invited_is_disposition_eleven() {
    let result = ChatIntentResult {
        disposition: ChatDisposition::NotInvited,
        wait_seconds: 0,
    };
    let bytes = encode_chat_intent_result(&result).expect("valid fixture");
    assert_eq!(bytes, [8, 11]);
    assert_eq!(decode_chat_intent_result(&bytes).expect("valid"), result);
}
