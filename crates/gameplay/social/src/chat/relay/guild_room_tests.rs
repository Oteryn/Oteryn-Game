#![allow(clippy::expect_used)]
use super::*;
use crate::domain::guild::{GuildId, GuildRank};

fn guild_line(rank: u8) -> RelayLine {
    RelayLine {
        destination: RelayDestination::Guild {
            guild_id: GuildId::from_bytes(*recipient(9).as_bytes()).expect("valid fixture"),
            rank: GuildRank::new(rank).expect("valid fixture"),
        },
        ..room_line(3, NOW)
    }
}

#[test]
fn guild_room_roundtrip_rank_and_destination_stability_across_nodes() {
    let ring = ring(&[(1, 0)]);
    for rank in [1, 2, 20] {
        let line = guild_line(rank);
        let sealed = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("valid fixture");
        let mut receiver = ReplayWindow::starting_at(NOW);
        assert_eq!(
            receiver
                .receive(&ring, CHANNEL, &sealed, NOW)
                .expect("valid fixture"),
            line
        );
        assert_eq!(
            receiver.receive(&ring, CHANNEL, &sealed, NOW),
            Err(RelayError::Replayed)
        );
        assert!(open(&ring, "oteryn_chat_foreign_world", &sealed, NOW).is_err());
        // The receiver only has payload GuildId and rank, never the sender's current guild.
        assert_eq!(
            sealed.len(),
            4 * (STANDARD
                .decode(&sealed)
                .expect("valid fixture")
                .len()
                .div_ceil(3))
        );
    }
}

#[test]
fn guild_room_malformed_truncated_extra_rank_uuid_and_tamper_fail_closed() {
    let ring = ring(&[(1, 0)]);
    let line = guild_line(20);
    let bytes = line.encode().expect("valid fixture");
    for end in 0..bytes.len() {
        assert!(RelayLine::decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(RelayLine::decode(&extra).is_err());
    for rank in [0, 21, 255] {
        let mut invalid = bytes.clone();
        invalid[41] = rank;
        assert!(RelayLine::decode(&invalid).is_err());
    }
    let mut uuid = bytes;
    uuid[31] = 0;
    assert!(RelayLine::decode(&uuid).is_err());
    let sealed = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("valid fixture");
    let mut raw = STANDARD.decode(sealed).expect("valid fixture");
    raw[20] ^= 1;
    assert_eq!(
        open(&ring, CHANNEL, &STANDARD.encode(raw), NOW),
        Err(RelayError::Unauthentic)
    );
}

#[test]
fn guild_room_old_private_and_room_plaintext_layouts_unchanged() {
    for destination in [
        RelayDestination::Private {
            recipient: recipient(9),
        },
        RelayDestination::Room {
            room: ChatRoom::English,
        },
    ] {
        let line = RelayLine {
            destination,
            ..room_line(1, NOW)
        };
        let mut expected = vec![if matches!(destination, RelayDestination::Private { .. }) {
            1
        } else {
            2
        }];
        expected.extend_from_slice(&[1; 16]);
        expected.extend_from_slice(&NOW.to_be_bytes());
        match destination {
            RelayDestination::Private { recipient } => {
                expected.extend_from_slice(recipient.as_bytes())
            }
            RelayDestination::Room { room } => expected.push(room as u8),
            _ => unreachable!(),
        }
        expected.extend_from_slice(&[0, 6]);
        expected.extend_from_slice(b"Sender");
        expected.extend_from_slice(&[0, 11]);
        expected.extend_from_slice(b"hello world");
        assert_eq!(line.encode().expect("valid fixture"), expected);
    }
}

#[test]
fn guild_room_maximum_bounds_key_overlap_freshness_and_clock_rollback() {
    assert_eq!(PLAINTEXT_MAX_BYTES, 1188);
    assert_eq!(SEALED_MAX_BYTES, 1217);
    let line = RelayLine {
        sender_name: name(&"n".repeat(29)),
        text: chat_text(&"\u{10348}".repeat(255)),
        ..guild_line(20)
    };
    let ring = ring(&[(1, 0), (2, NOW + 1)]);
    let sealed = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("valid fixture");
    assert!(sealed.len() <= 1624 && sealed.len() <= NOTIFY_MAX_BYTES);
    assert_eq!(
        open(&ring, CHANNEL, &sealed, NOW + FRESHNESS_MICROS),
        Ok(line)
    );
    assert_eq!(
        open(&ring, CHANNEL, &sealed, NOW + FRESHNESS_MICROS + 1),
        Err(RelayError::Stale)
    );
    assert_eq!(
        open(&ring, CHANNEL, &sealed, NOW + 1 + KEY_OVERLAP_MICROS),
        Err(RelayError::UnknownKey)
    );
    let mut window = ReplayWindow::starting_at(NOW);
    assert!(window.receive(&ring, CHANNEL, &sealed, NOW).is_ok());
    assert_eq!(
        window.receive(&ring, CHANNEL, &sealed, NOW - 1),
        Err(RelayError::Replayed)
    );
    window.reconnected(NOW);
    assert_eq!(
        window.receive(&ring, CHANNEL, &sealed, NOW),
        Err(RelayError::Stale)
    );
}
