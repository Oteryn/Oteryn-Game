#![allow(clippy::expect_used)]

use super::*;

const WORLD: &str = "01890a5d-ac96-7ab2-80e7-0123456789ab";
const CHANNEL: &str = "oteryn_chat_01890a5dac967ab280e70123456789ab";
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
        let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
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
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
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
            seal_with_nonce(&ring, CHANNEL, &line, NONCE),
            Err(RelayError::Oversized)
        );
    }
}

#[test]
fn another_channel_or_an_altered_line_fails_authentication() {
    let ring = ring(&[(1, 0)]);
    let text = seal_with_nonce(&ring, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    assert_eq!(
        open(
            &ring,
            "oteryn_chat_01890a5dac967ab280e70123456789ac",
            &text,
            NOW
        ),
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
fn a_new_key_is_sent_only_after_its_cluster_wide_activation() {
    let activate = NOW + 1_000;
    let updated = ring(&[(1, 0), (2, activate)]);
    let stale_node = ring(&[(1, 0)]);
    // Before activation the updated node still sends the old key every node holds.
    let before =
        seal_with_nonce(&updated, CHANNEL, &room_line(1, activate - 1), NONCE).expect("seal");
    assert_eq!(STANDARD.decode(&before).expect("base64")[0], 1);
    assert!(open(&stale_node, CHANNEL, &before, activate - 1).is_ok());
    // From activation it sends the new key; both keys stay openable on the updated node.
    let after = seal_with_nonce(&updated, CHANNEL, &room_line(2, activate), NONCE).expect("seal");
    assert_eq!(STANDARD.decode(&after).expect("base64")[0], 2);
    assert!(open(&updated, CHANNEL, &before, activate).is_ok());
    assert!(open(&updated, CHANNEL, &after, activate).is_ok());
    // No key activated yet at the line's time: nothing to send with.
    let future = self::ring(&[(3, activate)]);
    assert_eq!(
        seal_with_nonce(&future, CHANNEL, &room_line(3, activate - 1), NONCE),
        Err(RelayError::NoKey)
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
    let text = seal_with_nonce(&other, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
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
    let old_text = seal_with_nonce(&old_only, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
    let new_text = seal_with_nonce(&ring, CHANNEL, &room_line(2, NOW), NONCE).expect("seal");
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
    let text = seal_with_nonce(&ring, CHANNEL, &fresh, NONCE).expect("seal");
    assert_eq!(open(&ring, CHANNEL, &text, at_expiry), Ok(fresh));
}

#[test]
fn an_old_overlap_line_opens_while_fresh() {
    // Sealed by the old key just before rotation, opened inside the overlap.
    let old_only = ring(&[(1, 0)]);
    let both = ring(&[(1, 0), (2, NOW)]);
    let line = room_line(1, NOW);
    let text = seal_with_nonce(&old_only, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(open(&both, CHANNEL, &text, NOW + 1_000_000), Ok(line));
}

#[test]
fn a_line_outside_the_freshness_window_is_stale() {
    let ring = ring(&[(1, 0)]);
    let text = seal_with_nonce(&ring, CHANNEL, &room_line(1, NOW), NONCE).expect("seal");
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
    let mut window = ReplayWindow::starting_at(NOW);
    let line = room_line(1, NOW);
    assert_eq!(window.admit(&line, NOW), Ok(()));
    assert_eq!(window.admit(&line, NOW + 1), Err(RelayError::Replayed));
    assert_eq!(window.admit(&room_line(2, NOW), NOW), Ok(()));
}

#[test]
fn a_replay_is_still_refused_until_it_would_be_stale_anyway() {
    let mut window = ReplayWindow::starting_at(NOW);
    let ring = ring(&[(1, 0)]);
    let line = room_line(1, NOW);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
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
    assert!(window.expiry_of.len() == 1 && window.by_expiry.len() == 1);
}

#[test]
fn expiry_follows_each_lines_own_deadline_not_arrival_order() {
    let mut window = ReplayWindow::starting_at(NOW - FRESHNESS_MICROS);
    let late = room_line(1, NOW + 20_000_000);
    let early = room_line(2, NOW - 20_000_000);
    window.admit(&late, NOW).expect("late-stamped first");
    window.admit(&early, NOW).expect("early-stamped second");
    // `early` is no longer fresh after NOW + 10 s; it must be forgotten even though it arrived
    // behind `late`, while `late` is still remembered.
    let t = NOW + 10_000_001;
    assert_eq!(window.admit(&late, t), Err(RelayError::Replayed));
    assert_eq!(window.expiry_of.len(), 1);
    assert_eq!(window.by_expiry.len(), 1);
}

#[test]
fn a_clock_stepped_back_does_not_readmit_an_evicted_id() {
    let mut window = ReplayWindow::starting_at(NOW);
    let line = room_line(1, NOW);
    window.admit(&line, NOW).expect("first");
    let after = NOW + FRESHNESS_MICROS + 1;
    window
        .admit(&room_line(2, after), after)
        .expect("evicts the first");
    // The database clock steps back to when `line` was fresh; it must still be refused.
    assert_eq!(window.admit(&line, NOW), Err(RelayError::Stale));
}

#[test]
fn a_full_window_refuses_instead_of_forgetting() {
    let mut window = ReplayWindow::starting_at(NOW);
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
    assert_eq!(channel_name(WORLD).as_deref(), Ok(CHANNEL));
    assert_eq!(CHANNEL.len(), 44);
    for bad in [
        "",
        "world_a",
        "01890A5D-AC96-7AB2-80E7-0123456789AB",
        "01890a5dac967ab280e70123456789ab",
        "01890a5d-ac96-7ab2-80e7-0123456789a;",
        "01890a5d-ac96-7ab2-80e7-0123456789abc",
    ] {
        assert_eq!(channel_name(bad), Err(RelayError::BadChannel), "{bad}");
    }
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
    let ring = ring(&[(1, 0)]);
    let outgoing = || OutgoingLine {
        sent_at_micros: NOW,
        destination: RelayDestination::Room {
            room: ChatRoom::English,
        },
        sender_name: "Sender".to_owned(),
        text: "hello".to_owned(),
    };
    let a = open(
        &ring,
        CHANNEL,
        &seal(&ring, CHANNEL, outgoing()).expect("a"),
        NOW,
    )
    .expect("open a");
    let b = open(
        &ring,
        CHANNEL,
        &seal(&ring, CHANNEL, outgoing()).expect("b"),
        NOW,
    )
    .expect("open b");
    assert_ne!(a.message_id, b.message_id);
    let line = RelayLine {
        text: "secret words".to_owned(),
        ..room_line(1, NOW)
    };
    assert!(!format!("{line:?}").contains("secret"));
    assert!(!format!("{:?}", RelayKey::new(1, 0, [0xab; KEY_BYTES])).contains("171"));
}

#[test]
fn a_retired_key_stays_retired_when_the_clock_steps_back() {
    let added = NOW;
    let ring = ring(&[(1, 0), (2, added)]);
    let old_only = self::ring(&[(1, 0)]);
    let at_expiry = added + KEY_OVERLAP_MICROS;
    let old_text =
        seal_with_nonce(&old_only, CHANNEL, &room_line(1, at_expiry), NONCE).expect("seal");
    let mut window = ReplayWindow::starting_at(NOW);
    // Inside the overlap the old key still opens; at its expiry it is retired.
    assert!(
        window
            .receive(&ring, CHANNEL, &old_text, at_expiry - 1)
            .is_ok()
    );
    let other = seal_with_nonce(&old_only, CHANNEL, &room_line(2, at_expiry), NONCE).expect("seal");
    assert_eq!(
        window.receive(&ring, CHANNEL, &other, at_expiry),
        Err(RelayError::UnknownKey)
    );
    // The database clock steps back to before the expiry: the key must stay retired.
    assert_eq!(
        window.receive(&ring, CHANNEL, &other, at_expiry - 1),
        Err(RelayError::UnknownKey)
    );
    assert_eq!(
        window.receive(&ring, CHANNEL, &other, added),
        Err(RelayError::UnknownKey)
    );
}

#[test]
fn a_restart_does_not_make_a_captured_line_admissible_again() {
    let ring = ring(&[(1, 0)]);
    let line = room_line(1, NOW);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    let mut before = ReplayWindow::starting_at(NOW - 1);
    assert_eq!(before.receive(&ring, CHANNEL, &text, NOW), Ok(line));
    assert_eq!(
        before.receive(&ring, CHANNEL, &text, NOW + 1),
        Err(RelayError::Replayed)
    );
    // The node restarts within the freshness window; its new listener is ready after the line was
    // sent, and the replayed ciphertext is still inside the 30 s window.
    let ready = NOW + 5_000_000;
    let mut after = ReplayWindow::starting_at(startup_floor(ready));
    // The clock starts at the floor, so the old capture is refused as stale or as before the
    // start; either way it is not delivered.
    assert!(matches!(
        after.receive(&ring, CHANNEL, &text, ready + 1),
        Err(RelayError::Stale | RelayError::BeforeStart)
    ));
    // A line stamped from the floor on is admitted.
    let floor = startup_floor(ready);
    let fresh = room_line(2, floor);
    let fresh_text = seal_with_nonce(&ring, CHANNEL, &fresh, NONCE).expect("seal");
    assert_eq!(after.receive(&ring, CHANNEL, &fresh_text, floor), Ok(fresh));
}

#[test]
fn a_line_stamped_in_the_future_before_a_restart_is_not_delivered_again() {
    let ring = ring(&[(1, 0)]);
    // The old process admitted a line stamped the full future allowance ahead of its clock.
    let line = room_line(1, NOW + FRESHNESS_MICROS);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    let mut before = ReplayWindow::starting_at(NOW - 1);
    assert_eq!(before.receive(&ring, CHANNEL, &text, NOW), Ok(line));
    // It restarts at once, at the same database reading, and the capture is replayed.
    for ready in [NOW, NOW + 1_000_000, NOW + FRESHNESS_MICROS / 2] {
        let mut after = ReplayWindow::starting_at(startup_floor(ready));
        assert_eq!(
            after.receive(&ring, CHANNEL, &text, ready),
            Err(RelayError::BeforeStart),
            "ready at {ready}"
        );
    }
}

#[test]
fn the_startup_floor_depends_on_the_database_clock_only() {
    // The floor is a pure function of the database reading, whatever the host clock says.
    assert_eq!(startup_floor(NOW), NOW + FRESHNESS_MICROS + 1);
    assert_eq!(startup_floor(u64::MAX), u64::MAX);
    let ring = ring(&[(1, 0)]);
    let mut window = ReplayWindow::starting_at(startup_floor(NOW));
    let floor = startup_floor(NOW);
    let early = room_line(1, floor - 1);
    let early_text = seal_with_nonce(&ring, CHANNEL, &early, NONCE).expect("seal");
    assert_eq!(
        window.receive(&ring, CHANNEL, &early_text, floor),
        Err(RelayError::BeforeStart)
    );
    // Database time passes the floor: legitimate lines flow with no wall clock involved.
    let line = room_line(2, floor + 1);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(window.receive(&ring, CHANNEL, &text, floor + 1), Ok(line));
}

#[test]
fn a_retired_key_stays_retired_across_a_restart_and_a_clock_rollback() {
    let added = NOW;
    let ring = ring(&[(1, 0), (2, added)]);
    let old_only = self::ring(&[(1, 0)]);
    let at_expiry = added + KEY_OVERLAP_MICROS;
    let text = seal_with_nonce(&old_only, CHANNEL, &room_line(1, at_expiry), NONCE).expect("seal");
    // The process restarts after the expiry with the database clock rolled back before it; the
    // floor is taken from the reading at readiness, so the clock still starts past the expiry.
    let rolled_back = added;
    let mut window = ReplayWindow::starting_at(startup_floor(at_expiry));
    assert_eq!(
        window.receive(&ring, CHANNEL, &text, rolled_back),
        Err(RelayError::UnknownKey)
    );
}
