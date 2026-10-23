#![allow(clippy::expect_used)]
use super::*;
#[test]
fn guild_chat_roundtrip_and_strict_destination() {
    let mut guild_id = [1; 16];
    guild_id[6] = 0x70;
    guild_id[8] = 0x80;
    for rank in [1, 2, 20] {
        let line = ChatLine::Guild(GuildChatLine {
            guild_id,
            rank,
            speaker_name: "Knight".into(),
            text: "hello".into(),
        });
        let bytes = encode_chat_line(&line).expect("valid fixture");
        assert_eq!(decode_chat_line(&bytes).expect("valid fixture"), line);
        let snapshot = GuildChatSnapshot {
            rooms: ChatRoomSet::default(),
            guild: GuildRoomState::Member {
                guild_id,
                rank,
                motd: "Welcome".into(),
            },
        };
        assert_eq!(
            decode_snapshot(&encode_snapshot(&snapshot).expect("valid fixture"))
                .expect("valid fixture"),
            snapshot
        );
    }
    let intent = ChatIntent::Guild {
        text: "hello".into(),
    };
    assert_eq!(
        decode_chat_intent(&encode_chat_intent(&intent).expect("valid fixture"))
            .expect("valid fixture"),
        intent
    );
    for rank in [0, 21, 255] {
        assert!(
            encode_line(&GuildChatLine {
                guild_id,
                rank,
                speaker_name: "Knight".into(),
                text: "hello".into()
            })
            .is_err()
        );
    }
    for forged in [
        vec![16, 1],
        [vec![18, 16], guild_id.to_vec()].concat(),
        vec![26, 1, b'A'],
    ] {
        let mut inner = vec![10, 2, b'h', b'i'];
        inner.extend(forged);
        let mut payload = vec![50, u8::try_from(inner.len()).expect("bounded")];
        payload.extend(inner);
        assert!(decode_chat_intent(&payload).is_err());
    }

    assert_eq!(
        encode_chat_intent_result(&ChatIntentResult {
            disposition: ChatDisposition::NotAllowed,
            wait_seconds: 0
        })
        .expect("valid fixture"),
        [8, 10]
    );
}

#[test]
fn guild_chat_snapshot_maximum_and_strict_fields() {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut rooms = ChatRoomSet::default();
    for room in ChatRoom::ALL {
        rooms.insert(room);
    }
    let widest = GuildChatSnapshot {
        rooms,
        guild: GuildRoomState::Member {
            guild_id: id,
            rank: 20,
            motd: "\u{10348}".repeat(255),
        },
    };
    let bytes = encode_snapshot(&widest).expect("widest");
    assert_eq!(bytes.len(), MAX_GUILD_CHAT_SNAPSHOT_BYTES);
    assert_eq!(decode_snapshot(&bytes).expect("snapshot"), widest);
    for end in 0..bytes.len() {
        assert!(decode_snapshot(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.extend_from_slice(&[0x30, 0]);
    assert!(decode_snapshot(&extra).is_err());
    assert!(
        encode_snapshot(&GuildChatSnapshot {
            rooms,
            guild: GuildRoomState::Member {
                guild_id: id,
                rank: 1,
                motd: "a".repeat(256)
            }
        })
        .is_err()
    );
    assert!(decode_snapshot(&[10, 0, 16, 1, 26, 0]).is_err()); // absent with a guild id
}
