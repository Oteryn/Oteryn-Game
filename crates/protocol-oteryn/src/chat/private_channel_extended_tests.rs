#![allow(clippy::expect_used)]
use super::super::private_channel::InvitedChannel;
use super::*;

/// Every room open, the own channel and 16 invitations, all channel ids above 16,383 so each
/// takes three varint bytes.
fn full_list() -> ChatChannelList {
    let mut rooms = ChatRoomSet::default();
    for room in ChatRoom::ALL {
        rooms.insert(room);
    }
    ChatChannelList {
        public_rooms: rooms,
        own_channel: Some(u16::MAX),
        invited: (1..=MAX_INVITED_PRIVATE_CHANNELS)
            .map(|index| InvitedChannel {
                channel_id: u16::MAX - u16::try_from(index).expect("bounded"),
                owner_name: "N".repeat(MAX_CHAT_NAME_BYTES),
            })
            .collect(),
    }
}

fn ascending_ids(list: &ChatChannelList) -> Vec<u16> {
    let mut ids: Vec<u16> = list
        .invited
        .iter()
        .map(|invited| invited.channel_id)
        .chain(list.own_channel)
        .collect();
    ids.sort_unstable();
    ids
}

#[test]
fn the_largest_extended_channel_list_is_the_bound() {
    let list = full_list();
    let ids = ascending_ids(&list);
    let extension = ChannelListExtension {
        joined: ids.clone(),
        participants: ids
            .iter()
            .map(|channel_id| ChannelParticipants {
                channel_id: *channel_id,
                names: vec!["N".repeat(MAX_CHAT_NAME_BYTES); MAX_PRIVATE_CHANNEL_PARTICIPANTS],
            })
            .collect(),
    };
    let mut bytes = encode_channel_list_extended(&list, &extension).expect("valid fixture");
    assert_eq!(bytes.len(), MAX_CHAT_CHANNEL_LIST_EXTENDED_BYTES);
    assert_eq!(
        decode_channel_list_extended(&bytes).expect("valid fixture"),
        (list.clone(), extension.clone())
    );
    bytes.push(0);
    assert_eq!(
        decode_channel_list_extended(&bytes),
        Err(ChatWireError::LimitExceeded)
    );
    // An 18th name or channel is refused on both sides.
    let mut crowded = extension.clone();
    crowded.participants[0].names.push("A".into());
    assert_eq!(
        encode_channel_list_extended(&list, &crowded),
        Err(ChatWireError::LimitExceeded)
    );
    let mut too_many = extension;
    too_many.joined.push(1);
    assert_eq!(
        encode_channel_list_extended(&list, &too_many),
        Err(ChatWireError::LimitExceeded)
    );
}

#[test]
fn an_empty_extension_is_the_legacy_list() {
    for list in [full_list(), ChatChannelList::default()] {
        let bytes = encode_channel_list_extended(&list, &ChannelListExtension::default())
            .expect("valid fixture");
        assert_eq!(bytes, encode_channel_list(&list).expect("valid fixture"));
        assert_eq!(
            decode_channel_list_extended(&bytes).expect("valid fixture"),
            (list, ChannelListExtension::default())
        );
    }
}

#[test]
fn the_joined_and_participant_fields_are_four_and_five() {
    let list = ChatChannelList {
        public_rooms: ChatRoomSet::default(),
        own_channel: Some(5),
        invited: vec![InvitedChannel {
            channel_id: 9,
            owner_name: "Bryn".into(),
        }],
    };
    let extension = ChannelListExtension {
        joined: vec![5, 9],
        participants: vec![ChannelParticipants {
            channel_id: 9,
            names: vec!["Aldric".into(), "Bryn".into()],
        }],
    };
    let bytes = encode_channel_list_extended(&list, &extension).expect("valid fixture");
    let legacy = encode_channel_list(&list).expect("valid fixture");
    let mut tail = vec![32, 5, 32, 9, 42, 16, 8, 9, 18, 6];
    tail.extend_from_slice(b"Aldric");
    tail.extend_from_slice(&[18, 4]);
    tail.extend_from_slice(b"Bryn");
    assert_eq!(bytes, [legacy, tail].concat());
    assert_eq!(
        decode_channel_list_extended(&bytes).expect("valid fixture"),
        (list.clone(), extension)
    );
    // The legacy decoder never accepts fields 4 or 5, so an unselected session is never sent them.
    assert!(decode_channel_list(&bytes).is_err());
}

#[test]
fn the_extension_is_strict() {
    let list = ChatChannelList {
        public_rooms: ChatRoomSet::default(),
        own_channel: Some(5),
        invited: vec![InvitedChannel {
            channel_id: 9,
            owner_name: "Bryn".into(),
        }],
    };
    let legacy = encode_channel_list(&list).expect("valid fixture");
    // An unlisted, zero, repeated or descending channel; an entry without channel or name, an
    // empty or oversized name, an alien entry field and a wrong wire type all fail closed.
    for tail in [
        vec![32, 7],
        vec![32, 0],
        vec![32, 5, 32, 5],
        vec![32, 9, 32, 5],
        vec![42, 3, 18, 1, b'A'],
        vec![42, 2, 8, 9],
        vec![42, 4, 8, 9, 18, 0],
        vec![42, 5, 8, 9, 18, 1, 0xff],
        vec![42, 5, 8, 9, 26, 1, b'A'],
        vec![42, 5, 8, 7, 18, 1, b'A'],
        vec![42, 7, 8, 9, 8, 9, 18, 1, b'A'],
        vec![42, 5, 8, 9, 18, 1, b'A', 42, 5, 8, 9, 18, 1, b'B'],
        vec![34, 1, 5],
        vec![40, 9],
    ] {
        let payload = [legacy.clone(), tail].concat();
        assert!(
            decode_channel_list_extended(&payload).is_err(),
            "{payload:?}"
        );
    }
    let bad = ChannelListExtension {
        joined: vec![9, 5],
        participants: Vec::new(),
    };
    assert!(encode_channel_list_extended(&list, &bad).is_err());
    let unnamed = ChannelListExtension {
        joined: vec![5],
        participants: vec![ChannelParticipants {
            channel_id: 5,
            names: Vec::new(),
        }],
    };
    assert!(encode_channel_list_extended(&list, &unnamed).is_err());
    let long_name = ChannelListExtension {
        joined: Vec::new(),
        participants: vec![ChannelParticipants {
            channel_id: 5,
            names: vec!["N".repeat(MAX_CHAT_NAME_BYTES + 1)],
        }],
    };
    assert!(encode_channel_list_extended(&list, &long_name).is_err());
}
