#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/chat_v1.proto";
const SCHEMA: &str = include_str!("../../../docs/contracts/protocol-oteryn/v1/chat_v1.proto");

fn text(bytes: usize) -> String {
    "a".repeat(bytes)
}

fn local_line(name: usize, body: usize) -> ChatLine {
    ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [0xab; ENTITY_IDENTITY_BYTES],
            generation: u64::MAX,
        },
        speaker_name: text(name),
        mode: ChatSpeechMode::Whisper,
        text: text(body),
        position: ActorPosition {
            x: i32::MIN,
            y: i32::MIN,
            floor: 15,
        },
    }
}

fn round_trip_intent(intent: &ChatIntent) -> Vec<u8> {
    let bytes = encode_chat_intent(intent).expect("encode");
    assert_eq!(&decode_chat_intent(&bytes).expect("decode"), intent);
    bytes
}

fn round_trip_line(line: &ChatLine) -> Vec<u8> {
    let bytes = encode_chat_line(line).expect("encode");
    assert_eq!(&decode_chat_line(&bytes).expect("decode"), line);
    bytes
}

#[test]
fn intents_round_trip_and_the_private_message_is_the_byte_bound() {
    let say = ChatIntent::Say {
        mode: ChatSpeechMode::Say,
        text: "hi".to_owned(),
    };
    // 0a 06 | 08 01 | 12 02 'h' 'i'
    assert_eq!(
        round_trip_intent(&say),
        [0x0a, 0x06, 0x08, 0x01, 0x12, 0x02, b'h', b'i']
    );
    assert_eq!(
        round_trip_intent(&ChatIntent::OpenRoom(ChatRoom::Advertising)),
        [0x22, 0x02, 0x08, 0x04]
    );
    assert_eq!(
        round_trip_intent(&ChatIntent::CloseRoom(ChatRoom::World)),
        [0x2a, 0x02, 0x08, 0x01]
    );
    round_trip_intent(&ChatIntent::Room {
        room: ChatRoom::Help,
        text: text(MAX_CHAT_TEXT_BYTES),
    });
    let private = ChatIntent::Private {
        recipient_name: text(MAX_CHAT_NAME_BYTES),
        text: text(MAX_CHAT_TEXT_BYTES),
    };
    assert_eq!(round_trip_intent(&private).len(), MAX_CHAT_INTENT_BYTES);
    let yell = ChatIntent::Say {
        mode: ChatSpeechMode::Yell,
        text: "😀".repeat(255),
    };
    round_trip_intent(&yell);
}

#[test]
fn intents_outside_the_bounds_are_refused_in_both_directions() {
    for intent in [
        ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: text(MAX_CHAT_TEXT_BYTES + 1),
        },
        ChatIntent::Private {
            recipient_name: text(MAX_CHAT_NAME_BYTES + 1),
            text: "x".to_owned(),
        },
    ] {
        assert_eq!(
            encode_chat_intent(&intent),
            Err(ChatWireError::LimitExceeded)
        );
    }
    assert_eq!(
        encode_chat_intent(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: String::new(),
        }),
        Err(ChatWireError::Malformed)
    );
    // 1,021 bytes of text inside a say intent.
    let mut oversized = vec![0x0a, 0x82, 0x08, 0x08, 0x01, 0x12, 0xfd, 0x07];
    oversized.extend(std::iter::repeat_n(b'a', 1_021));
    assert_eq!(
        decode_chat_intent(&oversized),
        Err(ChatWireError::LimitExceeded)
    );
    let mut too_long = vec![0u8; MAX_CHAT_INTENT_BYTES + 1];
    too_long[0] = 0x0a;
    assert_eq!(
        decode_chat_intent(&too_long),
        Err(ChatWireError::LimitExceeded)
    );
}

#[test]
fn malformed_intents_fail_closed() {
    for payload in [
        &[][..],                                                       // empty oneof
        &[0x0a, 0x02, 0x08, 0x01],                                     // say without text
        &[0x0a, 0x05, 0x08, 0x00, 0x12, 0x01, b'x'],                   // mode 0
        &[0x0a, 0x05, 0x08, 0x04, 0x12, 0x01, b'x'],                   // unknown mode
        &[0x0a, 0x03, 0x12, 0x01, b'x'],                               // say without mode
        &[0x0a, 0x04, 0x08, 0x01, 0x12, 0x00],                         // empty text
        &[0x0a, 0x05, 0x08, 0x01, 0x12, 0x01, 0xff],                   // invalid UTF-8
        &[0x22, 0x02, 0x08, 0x05],                                     // unknown room
        &[0x22, 0x00],                                                 // open_room without room
        &[0x22, 0x05, 0x08, 0x01, 0x12, 0x01, b'x'],                   // text on open_room
        &[0x12, 0x05, 0x08, 0x01, 0x12, 0x01, b'x'], // private with an enum, no name
        &[0x22, 0x02, 0x08, 0x01, 0x22, 0x02, 0x08, 0x02], // two intents
        &[0x32, 0x02, 0x08, 0x01],                   // field 6
        &[0x08, 0x01],                               // varint at the top level
        &[0x0a, 0x08, 0x08, 0x01, 0x08, 0x01, 0x12, 0x01, b'x', 0x00], // repeated mode, junk
    ] {
        assert_eq!(
            decode_chat_intent(payload),
            Err(ChatWireError::Malformed),
            "{payload:02x?}"
        );
    }
}

#[test]
fn results_carry_a_wait_only_with_muted_and_exhausted() {
    for (result, bytes) in [
        (
            ChatIntentResult {
                disposition: ChatDisposition::Ok,
                wait_seconds: 0,
            },
            vec![0x08, 0x01],
        ),
        (
            ChatIntentResult {
                disposition: ChatDisposition::Muted,
                wait_seconds: MAX_CHAT_WAIT_SECONDS,
            },
            vec![0x08, 0x02, 0x10, 0x80, 0x0a],
        ),
        (
            ChatIntentResult {
                disposition: ChatDisposition::Rejected,
                wait_seconds: 0,
            },
            vec![0x08, 0x09],
        ),
    ] {
        let encoded = encode_chat_intent_result(&result).expect("encode");
        assert_eq!(encoded, bytes);
        assert_eq!(decode_chat_intent_result(&encoded), Ok(result));
    }
    let refused = [
        (ChatDisposition::Muted, 0, ChatWireError::Malformed),
        (ChatDisposition::Exhausted, 0, ChatWireError::Malformed),
        (ChatDisposition::Ok, 5, ChatWireError::Malformed),
        (
            ChatDisposition::Exhausted,
            MAX_CHAT_WAIT_SECONDS + 1,
            ChatWireError::LimitExceeded,
        ),
    ];
    for (disposition, wait_seconds, error) in refused {
        let result = ChatIntentResult {
            disposition,
            wait_seconds,
        };
        assert_eq!(encode_chat_intent_result(&result), Err(error));
    }
    for payload in [
        &[][..],
        &[0x08, 0x00],
        &[0x08, 0x0a],
        &[0x08, 0x02],
        &[0x08, 0x01, 0x10, 0x01],
        &[0x08, 0x01, 0x18, 0x01],
    ] {
        assert_eq!(
            decode_chat_intent_result(payload),
            Err(ChatWireError::Malformed),
            "{payload:02x?}"
        );
    }
}

#[test]
fn the_room_set_is_four_bits() {
    let mut rooms = ChatRoomSet::default();
    assert_eq!(encode_chat_rooms(rooms), Vec::<u8>::new());
    assert_eq!(decode_chat_rooms(&[]), Ok(rooms));
    for room in ChatRoom::ALL {
        rooms.insert(room);
    }
    let all = encode_chat_rooms(rooms);
    assert_eq!(all, [0x08, 0x0f]);
    assert!(all.len() <= MAX_CHAT_ROOMS_BYTES);
    assert_eq!(decode_chat_rooms(&all), Ok(rooms));
    rooms.remove(ChatRoom::English);
    assert!(!rooms.contains(ChatRoom::English));
    assert!(rooms.contains(ChatRoom::Advertising));
    assert_eq!(encode_chat_rooms(rooms), [0x08, 0x0d]);
    for payload in [
        &[0x08, 0x10][..],
        &[0x08, 0x1f],
        &[0x08, 0x80, 0x02],
        &[0x10, 0x01],
    ] {
        assert_eq!(
            decode_chat_rooms(payload),
            Err(ChatWireError::Malformed),
            "{payload:02x?}"
        );
    }
}

#[test]
fn lines_round_trip_and_the_local_line_is_the_byte_bound() {
    assert_eq!(
        round_trip_line(&local_line(MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES)).len(),
        MAX_CHAT_LINE_BYTES
    );
    let at_origin = ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [1; ENTITY_IDENTITY_BYTES],
            generation: 0,
        },
        speaker_name: "Bob".to_owned(),
        mode: ChatSpeechMode::Say,
        text: "hi".to_owned(),
        position: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    };
    // An empty position is still present on a local line.
    assert!(round_trip_line(&at_origin).ends_with(&[0x42, 0x00]));
    assert_eq!(
        round_trip_line(&ChatLine::Private {
            speaker_name: "Bob".to_owned(),
            text: "hi".to_owned(),
        }),
        [
            0x08, 0x02, 0x22, 0x03, b'B', b'o', b'b', 0x3a, 0x02, b'h', b'i'
        ]
    );
    round_trip_line(&ChatLine::Room {
        room: ChatRoom::World,
        speaker_name: text(MAX_CHAT_NAME_BYTES),
        text: text(MAX_CHAT_TEXT_BYTES),
    });
    assert_eq!(round_trip_line(&ChatLine::Dropped), [0x08, 0x04]);
}

#[test]
fn lines_with_fields_of_another_kind_or_out_of_bounds_are_refused() {
    assert_eq!(
        encode_chat_line(&local_line(MAX_CHAT_NAME_BYTES + 1, 1)),
        Err(ChatWireError::LimitExceeded)
    );
    assert_eq!(
        encode_chat_line(&local_line(1, MAX_CHAT_TEXT_BYTES + 1)),
        Err(ChatWireError::LimitExceeded)
    );
    let mut underground = local_line(1, 1);
    if let ChatLine::Local { position, .. } = &mut underground {
        position.floor = 16;
    }
    assert_eq!(
        encode_chat_line(&underground),
        Err(ChatWireError::Malformed)
    );
    let private = [0x08, 0x02, 0x22, 0x01, b'B', 0x3a, 0x01, b'x'];
    let mut with_mode = private.to_vec();
    with_mode.extend([0x28, 0x01]);
    let mut with_room = private.to_vec();
    with_room.extend([0x30, 0x01]);
    let mut dropped_with_text = vec![0x08, 0x04];
    dropped_with_text.extend([0x3a, 0x01, b'x']);
    for payload in [
        &[][..],
        &[0x08, 0x00],
        &[0x08, 0x05],
        &[0x08, 0x02, 0x22, 0x01, b'B'], // private without text
        &[0x08, 0x02, 0x3a, 0x01, b'x'], // private without name
        &[0x08, 0x03, 0x22, 0x01, b'B', 0x3a, 0x01, b'x'], // room without room
        &[0x08, 0x01, 0x22, 0x01, b'B', 0x28, 0x01, 0x3a, 0x01, b'x'], // local without identity
        &[0x08, 0x02, 0x08, 0x02, 0x22, 0x01, b'B', 0x3a, 0x01, b'x'], // repeated kind
        &with_mode,
        &with_room,
        &dropped_with_text,
    ] {
        assert_eq!(
            decode_chat_line(payload),
            Err(ChatWireError::Malformed),
            "{payload:02x?}"
        );
    }
    let mut short_identity = encode_chat_line(&local_line(1, 1)).expect("line");
    // 08 01 | 12 10 <16 bytes>: shorten the identity to 15 bytes.
    short_identity[3] = 0x0f;
    short_identity.remove(4);
    assert_eq!(
        decode_chat_line(&short_identity),
        Err(ChatWireError::Malformed)
    );
    let mut oversized = vec![0u8; MAX_CHAT_LINE_BYTES + 1];
    oversized[0] = 0x08;
    assert_eq!(
        decode_chat_line(&oversized),
        Err(ChatWireError::LimitExceeded)
    );
}

/// The IDs are registered exactly once, under these names, with the codecs' bounds; the proto
/// states the same messages and bounds.
#[test]
fn registries_bind_the_chat_wire_ids_and_limits() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let only = |section: &str, id: u32| {
        let matches: Vec<&Value> = protocol[section]
            .as_array()
            .expect(section)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{section} {id} registered once");
        matches[0].clone()
    };

    let capability = only("capabilities", CAPABILITY_CHAT_V1);
    assert_eq!(capability["name"], "CHAT_V1");
    assert_eq!(capability["offered"], false, "not offered before CHAT-1b-2");
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_CHAT_INTENT])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_CHAT])
    );

    let command = only("command_types", COMMAND_TYPE_CHAT_INTENT);
    assert_eq!(command["name"], "CHAT_INTENT");
    assert_eq!(command["capability"], CAPABILITY_CHAT_V1);
    assert_eq!(
        command["payload_schema"],
        format!("{SCHEMA_PATH}#ChatIntentV1")
    );
    assert_eq!(
        command["result_schema"],
        format!("{SCHEMA_PATH}#ChatIntentResultV1")
    );
    assert_eq!(command["max_payload_bytes"], MAX_CHAT_INTENT_BYTES as u64);
    assert_eq!(
        command["max_result_payload_bytes"],
        MAX_CHAT_RESULT_BYTES as u64
    );

    let domain = only("state_domains", STATE_DOMAIN_CHAT);
    assert_eq!(domain["name"], "CHAT");
    assert_eq!(domain["capability"], CAPABILITY_CHAT_V1);
    let types = |section: &str| -> Vec<(u64, String, String, u64)> {
        domain[section]
            .as_array()
            .expect(section)
            .iter()
            .map(|entry| {
                (
                    entry["id"].as_u64().expect("id"),
                    entry["name"].as_str().expect("name").to_owned(),
                    entry["payload_schema"].as_str().expect("schema").to_owned(),
                    entry["max_payload_bytes"].as_u64().expect("bound"),
                )
            })
            .collect()
    };
    let schema = |message: &str| format!("{SCHEMA_PATH}#{message}");
    assert_eq!(
        types("delta_types"),
        [
            (
                u64::from(DELTA_TYPE_CHAT_LINE_V1),
                "CHAT_LINE_DELTA_V1".to_owned(),
                schema("ChatLineV1"),
                MAX_CHAT_LINE_BYTES as u64
            ),
            (
                u64::from(DELTA_TYPE_CHAT_ROOMS_V1),
                "CHAT_ROOMS_DELTA_V1".to_owned(),
                schema("ChatRoomsV1"),
                MAX_CHAT_ROOMS_BYTES as u64
            ),
        ]
    );
    assert_eq!(
        types("snapshot_types"),
        [(
            u64::from(SNAPSHOT_TYPE_CHAT_V1),
            "CHAT_SNAPSHOT_V1".to_owned(),
            schema("ChatRoomsV1"),
            MAX_CHAT_ROOMS_BYTES as u64
        )]
    );

    for message in [
        "ChatSayV1",
        "ChatPrivateV1",
        "ChatRoomLineV1",
        "ChatRoomRefV1",
        "ChatIntentV1",
        "ChatIntentResultV1",
        "ChatRoomsV1",
        "ChatPositionV1",
        "ChatLineV1",
    ] {
        assert!(
            SCHEMA.contains(&format!("message {message} {{")),
            "{message}"
        );
    }
    for bound in [
        MAX_CHAT_INTENT_BYTES,
        MAX_CHAT_RESULT_BYTES,
        MAX_CHAT_ROOMS_BYTES,
        MAX_CHAT_LINE_BYTES,
    ] {
        assert!(
            SCHEMA.contains(&format!("At most {bound} bytes")),
            "{bound}"
        );
    }
    assert!(SCHEMA.contains(&format!("1..={MAX_CHAT_WAIT_SECONDS}")));
    assert!(SCHEMA.contains(&format!("1..={MAX_CHAT_TEXT_BYTES} bytes")));
    assert!(SCHEMA.contains(&format!("1..={MAX_CHAT_NAME_BYTES} bytes")));

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let entries = resources["entries"].as_array().expect("entries");
    let limit = |id: &str| {
        let matches: Vec<&Value> = entries.iter().filter(|entry| entry["id"] == id).collect();
        assert_eq!(matches.len(), 1, "{id} registered once");
        matches[0]["hard_maximum"].as_u64()
    };
    assert_eq!(limit("CHAT0-RL-01"), Some(MAX_CHAT_TEXT_BYTES as u64));
    assert_eq!(limit("CHAT0-RL-07"), Some(MAX_CHAT_OPEN_ROOMS as u64));
}
