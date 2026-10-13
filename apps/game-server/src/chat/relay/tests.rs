#![allow(clippy::expect_used)]

use super::*;

const CHANNEL: &str = "oteryn_chat_world_a";
const NOW: u64 = 1_000_000_000_000;
const NONCE: [u8; NONCE_BYTES] = [7; NONCE_BYTES];

fn ring(keys: &[(u8, u64)]) -> RelayKeyRing {
    RelayKeyRing::new(
        keys.iter()
            .map(|&(id, at)| RelayKey::new(id, at, [id; KEY_BYTES]))
            .collect(),
    )
    .expect("ring")
}

fn room_line(id: u8, sent_at: u64) -> RelayLine {
    RelayLine {
        message_id: [id; ID_BYTES],
        sent_at_micros: sent_at,
        destination: RelayDestination::Room {
            room: ChatRoom::English,
        },
        sender_name: "Sender".to_owned(),
        text: "hello world".to_owned(),
    }
}

#[test]
fn a_room_and_a_private_line_round_trip() {
    let ring = ring(&[(1, 0)]);
    let room = room_line(1, NOW);
    let private = RelayLine {
        destination: RelayDestination::Private {
            recipient: [9; ID_BYTES],
        },
        ..room_line(2, NOW)
    };
    for line in [room, private] {
        let text = seal(&ring, CHANNEL, &line, NONCE).expect("seal");
        assert_eq!(open(&ring, CHANNEL, &text, NOW), Ok(line));
    }
}

#[test]
fn the_widest_line_stays_inside_the_documented_bounds() {
    let ring = ring(&[(1, 0)]);
    let line = RelayLine {
        destination: RelayDestination::Private {
            recipient: [9; ID_BYTES],
        },
        sender_name: "n".repeat(SENDER_NAME_MAX_BYTES),
        text: "ą".repeat(TEXT_MAX_BYTES / 2),
        ..room_line(1, NOW)
    };
    let text = seal(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert!(text.len() <= 1_624, "{}", text.len());
    assert!(STANDARD.decode(&text).expect("base64").len() <= 1_216);
    assert_eq!(SEALED_MAX_BYTES, 1_214);
    assert_eq!(open(&ring, CHANNEL, &text, NOW), Ok(line));
}

#[test]
fn an_oversized_field_is_rejected_not_truncated() {
    let ring = ring(&[(1, 0)]);
    let long_text = RelayLine {
        text: "a".repeat(TEXT_MAX_BYTES + 1),
        ..room_line(1, NOW)
    };
    let long_name = RelayLine {
        sender_name: "n".repeat(SENDER_NAME_MAX_BYTES + 1),
        ..room_line(1, NOW)
    };
    for line in [long_text, long_name] {
        assert_eq!(
            seal(&ring, CHANNEL, &line, NONCE),
            Err(RelayError::Oversized)
        );
    }
}

#[test]
fn another_channel_or_an_altered_line_fails_authentication() {
    let ring = ring(&[(1, 0)]);
    let text = seal(&ring, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    assert_eq!(
        open(&ring, "oteryn_chat_world_b", &text, NOW),
        Err(RelayError::Unauthentic)
    );
    let mut bytes = STANDARD.decode(&text).expect("base64");
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(
        open(&ring, CHANNEL, &STANDARD.encode(&bytes), NOW),
        Err(RelayError::Unauthentic)
    );
    // The key_id is associated data: relabelling a line to another held key fails too.
    let two = RelayKeyRing::new(vec![
        RelayKey::new(1, 0, [1; KEY_BYTES]),
        RelayKey::new(2, 10, [1; KEY_BYTES]),
    ])
    .expect("ring");
    let mut bytes = STANDARD.decode(&text).expect("base64");
    bytes[0] = 2;
    assert_eq!(
        open(&two, CHANNEL, &STANDARD.encode(&bytes), NOW),
        Err(RelayError::Unauthentic)
    );
}

#[test]
fn garbage_is_malformed_and_an_unheld_key_is_unknown() {
    let ring = ring(&[(1, 0)]);
    assert_eq!(open(&ring, CHANNEL, "!!!", NOW), Err(RelayError::Malformed));
    assert_eq!(
        open(&ring, CHANNEL, &STANDARD.encode([1_u8; 5]), NOW),
        Err(RelayError::Malformed)
    );
    assert_eq!(
        open(&ring, CHANNEL, &"A".repeat(NOTIFY_MAX_BYTES + 1), NOW),
        Err(RelayError::Malformed)
    );
    let other = self::ring(&[(5, 0)]);
    let text = seal(&other, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    assert_eq!(
        open(&ring, CHANNEL, &text, NOW),
        Err(RelayError::UnknownKey)
    );
}

#[test]
fn rotation_sends_with_the_newest_key_and_opens_the_old_one_for_24_hours() {
    let added = NOW;
    let ring = ring(&[(1, 0), (2, added)]);
    let old_only = self::ring(&[(1, 0)]);
    let old_text = seal(&old_only, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    let new_text = seal(&ring, CHANNEL, &room_line(2, NOW), NONCE).expect("seal");
    assert_eq!(STANDARD.decode(&new_text).expect("base64")[0], 2);

    let just_inside = added + KEY_OVERLAP_MICROS - 1;
    assert!(
        open(&ring, CHANNEL, &old_text, just_inside).is_err_and(|e| e != RelayError::UnknownKey)
    );
    let at_expiry = added + KEY_OVERLAP_MICROS;
    assert_eq!(
        open(&ring, CHANNEL, &old_text, at_expiry),
        Err(RelayError::UnknownKey)
    );
    // The newest key never expires.
    let fresh = room_line(3, at_expiry);
    let text = seal(&ring, CHANNEL, &fresh, NONCE).expect("seal");
    assert_eq!(open(&ring, CHANNEL, &text, at_expiry), Ok(fresh));
}

#[test]
fn an_old_overlap_line_opens_while_fresh() {
    // Sealed by the old key just before rotation, opened inside the overlap.
    let old_only = ring(&[(1, 0)]);
    let both = ring(&[(1, 0), (2, NOW)]);
    let line = room_line(1, NOW);
    let text = seal(&old_only, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(open(&both, CHANNEL, &text, NOW + 1_000_000), Ok(line));
}

#[test]
fn a_line_outside_the_freshness_window_is_stale() {
    let ring = ring(&[(1, 0)]);
    let text = seal(&ring, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    assert!(open(&ring, CHANNEL, &text, NOW + FRESHNESS_MICROS).is_ok());
    assert_eq!(
        open(&ring, CHANNEL, &text, NOW + FRESHNESS_MICROS + 1),
        Err(RelayError::Stale)
    );
    assert_eq!(
        open(&ring, CHANNEL, &text, NOW - FRESHNESS_MICROS - 1),
        Err(RelayError::Stale)
    );
}

#[test]
fn a_message_id_is_admitted_once_inside_the_window() {
    let mut window = ReplayWindow::default();
    let line = room_line(1, NOW);
    assert_eq!(window.admit(&line, NOW), Ok(()));
    assert_eq!(window.admit(&line, NOW + 1), Err(RelayError::Replayed));
    assert_eq!(window.admit(&room_line(2, NOW), NOW), Ok(()));
}

#[test]
fn a_replay_is_still_refused_until_it_would_be_stale_anyway() {
    let mut window = ReplayWindow::default();
    let ring = ring(&[(1, 0)]);
    let line = room_line(1, NOW);
    let text = seal(&ring, CHANNEL, &line, NONCE).expect("seal");
    window.admit(&line, NOW).expect("first");
    // Every instant at which `open` still accepts the line, the window still remembers it.
    let last_fresh = NOW + FRESHNESS_MICROS;
    let reopened = open(&ring, CHANNEL, &text, last_fresh).expect("fresh");
    assert_eq!(
        window.admit(&reopened, last_fresh),
        Err(RelayError::Replayed)
    );
    let later = NOW + FRESHNESS_MICROS * 2 + 1;
    assert_eq!(open(&ring, CHANNEL, &text, later), Err(RelayError::Stale));
    assert_eq!(window.admit(&room_line(9, later), later), Ok(()));
    assert!(window.seen.len() == 1 && window.order.len() == 1);
}

#[test]
fn a_full_window_refuses_instead_of_forgetting() {
    let mut window = ReplayWindow::default();
    for n in 0..REPLAY_WINDOW_LINES {
        let mut line = room_line(0, NOW);
        line.message_id[..8].copy_from_slice(&(n as u64).to_be_bytes());
        assert_eq!(window.admit(&line, NOW), Ok(()));
    }
    assert_eq!(
        window.admit(&room_line(0xff, NOW), NOW),
        Err(RelayError::Saturated)
    );
}

#[test]
fn the_channel_name_is_a_safe_world_scoped_identifier() {
    assert_eq!(channel_name("world_a").as_deref(), Ok(CHANNEL));
    for bad in ["", "World", "a-b", "a b", "a;b", &"w".repeat(52)] {
        assert_eq!(channel_name(bad), Err(RelayError::BadChannel), "{bad}");
    }
    assert!(channel_name(&"w".repeat(51)).is_ok());
}

#[test]
fn a_ring_needs_distinct_key_ids() {
    assert_eq!(
        RelayKeyRing::new(vec![]).expect_err("refused"),
        RelayError::NoKey
    );
    let dup = vec![
        RelayKey::new(1, 0, [1; KEY_BYTES]),
        RelayKey::new(1, 5, [2; KEY_BYTES]),
    ];
    assert_eq!(
        RelayKeyRing::new(dup).expect_err("refused"),
        RelayError::NoKey
    );
}

#[test]
fn randomness_differs_and_debug_never_shows_text_or_key() {
    assert_ne!(random_nonce(), random_nonce());
    assert_ne!(random_message_id(), random_message_id());
    let line = RelayLine {
        text: "secret words".to_owned(),
        ..room_line(1, NOW)
    };
    assert!(!format!("{line:?}").contains("secret"));
    assert!(!format!("{:?}", RelayKey::new(1, 0, [0xab; KEY_BYTES])).contains("171"));
}
