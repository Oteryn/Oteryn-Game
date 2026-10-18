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

fn chat_text(text: &str) -> ChatText {
    ChatText::parse(text).expect("valid chat text")
}

fn recipient(marker: u8) -> CharacterId {
    let mut bytes = [marker; ID_BYTES];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    CharacterId::decode(&bytes).expect("valid UUIDv7 character id")
}

fn name(value: &str) -> CharacterName {
    CharacterName::parse(value).expect("valid character name")
}

fn room_line(id: u8, sent_at: u64) -> RelayLine {
    RelayLine {
        message_id: [id; ID_BYTES],
        sent_at_micros: sent_at,
        destination: RelayDestination::Room {
            room: ChatRoom::English,
        },
        sender_name: name("Sender"),
        text: chat_text("hello world"),
    }
}

#[test]
fn a_room_and_a_private_line_round_trip() {
    let ring = ring(&[(1, 0)]);
    let room = room_line(1, NOW);
    let private = RelayLine {
        destination: RelayDestination::Private {
            recipient: recipient(9),
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
            recipient: recipient(9),
        },
        sender_name: name(&"n".repeat(29)),
        // 255 four-byte scalars: both the character and the byte bound of CHAT-0 §4.
        text: chat_text(&"\u{10348}".repeat(255)),
        ..room_line(1, NOW)
    };
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert!(text.len() <= 1_624, "{}", text.len());
    assert!(STANDARD.decode(&text).expect("base64").len() <= 1_216);
    assert_eq!(SEALED_MAX_BYTES, 1_214);
    assert_eq!(open(&ring, CHANNEL, &text, NOW), Ok(line));
}

#[test]
fn a_received_sender_name_the_naming_policy_refuses_is_a_bad_payload() {
    let ring = ring(&[(1, 0)]);
    let valid = room_line(1, NOW);
    let plaintext = valid.encode().expect("encode");
    let name_at = 1 + ID_BYTES + 8 + 1;
    let tail = name_at + 2 + valid.sender_name.as_str().len();
    for bad in [
        "",
        "A",
        " Sender",
        "Sender ",
        "Se  nder",
        "Sender1",
        "Send\u{e9}r",
        &"n".repeat(30),
    ] {
        let mut forged = plaintext[..name_at].to_vec();
        push_prefixed(&mut forged, bad.as_bytes()).expect("prefixed");
        forged.extend_from_slice(&plaintext[tail..]);
        let sealed = seal_plaintext(&ring, CHANNEL, NOW, forged, NONCE).expect("seal");
        assert_eq!(
            open(&ring, CHANNEL, &sealed, NOW),
            Err(RelayError::BadPayload),
            "{bad:?}"
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
    for variant in ['8', '9', 'a', 'b'] {
        let world = format!("01890a5d-ac96-7ab2-{variant}0e7-0123456789ab");
        assert_eq!(
            channel_name(&world),
            Ok(format!(
                "oteryn_chat_01890a5dac967ab2{variant}0e70123456789ab"
            )),
            "{world}"
        );
    }
    for bad in [
        "",
        "world_a",
        "01890A5D-AC96-7AB2-80E7-0123456789AB",
        "01890a5dac967ab280e70123456789ab",
        "01890a5d-ac96-7ab2-80e7-0123456789a;",
        "01890a5d-ac96-7ab2-80e7-0123456789abc",
        // Shape-valid but not UUIDv7: version nibble 4, 1 or 0.
        "01890a5d-ac96-4ab2-80e7-0123456789ab",
        "01890a5d-ac96-1ab2-80e7-0123456789ab",
        "01890a5d-ac96-0ab2-80e7-0123456789ab",
        // Version 7 but not the RFC 9562 variant: nibble 0, 7, c or f.
        "01890a5d-ac96-7ab2-00e7-0123456789ab",
        "01890a5d-ac96-7ab2-70e7-0123456789ab",
        "01890a5d-ac96-7ab2-c0e7-0123456789ab",
        "01890a5d-ac96-7ab2-f0e7-0123456789ab",
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
        sender_name: name("Sender"),
        text: chat_text("hello"),
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
        text: chat_text("secret words"),
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

/// Stores the lease a listener owes at `db_now`, if any, and returns the lease now stored.
fn renew(window: &mut ReplayWindow, db_now: u64) -> u64 {
    if let Some(lease) = window.lease_due(db_now) {
        window.lease_stored(lease);
    }
    window.leased_until.expect("a lease is stored")
}

/// A window resumed after a restart with its first lease stored, as a listener runs it.
fn resumed(db_now: u64, stored: Option<u64>) -> (ReplayWindow, u64) {
    let mut window = ReplayWindow::resume(db_now, stored);
    assert!(
        window.lease_due(db_now).is_some(),
        "a new window owes a lease"
    );
    let lease = renew(&mut window, db_now);
    (window, lease)
}

#[test]
fn a_window_receives_only_inside_a_stored_lease() {
    let ring = ring(&[(1, 0)]);
    let mut window = ReplayWindow::resume(NOW, None);
    let floor = startup_floor(NOW);
    let line = room_line(1, floor);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    // Nothing is stored yet: the line is dropped before any key is consulted.
    assert_eq!(
        window.receive(&ring, CHANNEL, &text, floor),
        Err(RelayError::Unleased)
    );
    let lease = window.lease_due(floor).expect("due");
    assert_eq!(lease, floor + RELAY_LEASE_MICROS);
    window.lease_stored(lease);
    assert_eq!(window.lease_due(floor), None);
    assert_eq!(window.receive(&ring, CHANNEL, &text, floor), Ok(line));
    // Past the stored lease the window stops until a new one is stored.
    let later = room_line(2, lease + 1);
    let later_text = seal_with_nonce(&ring, CHANNEL, &later, NONCE).expect("seal");
    assert_eq!(
        window.receive(&ring, CHANNEL, &later_text, lease + 1),
        Err(RelayError::Unleased)
    );
    let renewed = window.lease_due(lease + 1).expect("due again");
    window.lease_stored(renewed);
    // A lower value reported later never shrinks the lease.
    window.lease_stored(lease);
    assert_eq!(
        window.receive(&ring, CHANNEL, &later_text, lease + 1),
        Ok(later)
    );
    assert_eq!(window.leased_until, Some(renewed));
}

#[test]
fn a_retired_key_stays_retired_across_a_restart_and_a_clock_rollback() {
    let added = NOW;
    let ring = ring(&[(1, 0), (2, added)]);
    let old_only = self::ring(&[(1, 0)]);
    let at_expiry = added + KEY_OVERLAP_MICROS;
    // The old process sees the old key retire, under a lease stored at that reading.
    let (mut before, _) = resumed(at_expiry - FRESHNESS_MICROS - 1, None);
    let stored = renew(&mut before, at_expiry);
    let probe = seal_with_nonce(&old_only, CHANNEL, &room_line(1, at_expiry), NONCE).expect("seal");
    assert_eq!(
        before.receive(&ring, CHANNEL, &probe, at_expiry),
        Err(RelayError::UnknownKey)
    );
    // The database clock is rolled back to before the expiry and the node restarts: the readiness
    // reading itself is rolled back, and a holder of the old key forges a line for that time.
    for rolled_back in [added, at_expiry - FRESHNESS_MICROS, at_expiry - 1] {
        let (mut after, _) = resumed(rolled_back, Some(stored));
        let forged =
            seal_with_nonce(&old_only, CHANNEL, &room_line(2, rolled_back), NONCE).expect("seal");
        // The rolled-back reading is far behind the resumed clock, so no lease covers it.
        assert!(
            matches!(
                after.receive(&ring, CHANNEL, &forged, rolled_back),
                Err(RelayError::Unleased | RelayError::UnknownKey)
            ),
            "rolled back to {rolled_back}"
        );
        // Once a lease covers the resumed clock the old key is still retired.
        let floor = after.floor;
        renew(&mut after, floor);
        let late = seal_with_nonce(&old_only, CHANNEL, &room_line(3, floor), NONCE).expect("seal");
        assert_eq!(
            after.receive(&ring, CHANNEL, &late, floor),
            Err(RelayError::UnknownKey),
            "rolled back to {rolled_back}"
        );
    }
}

#[test]
fn a_future_stamped_line_is_not_delivered_again_after_a_restart_and_a_clock_rollback() {
    let ring = ring(&[(1, 0)]);
    // The old process admits a line stamped the full future allowance ahead of its clock.
    let (mut before, _) = resumed(NOW - FRESHNESS_MICROS - 1, None);
    let stored = renew(&mut before, NOW);
    let line = room_line(1, NOW + FRESHNESS_MICROS);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(before.receive(&ring, CHANNEL, &text, NOW), Ok(line));
    // It restarts at once, with the database clock at, or rolled back before, the old reading.
    for ready in [NOW - FRESHNESS_MICROS, NOW - 1, NOW] {
        let (mut after, _) = resumed(ready, Some(stored));
        assert!(
            matches!(
                after.receive(&ring, CHANNEL, &text, ready),
                Err(RelayError::Stale | RelayError::BeforeStart | RelayError::Unleased)
            ),
            "ready at {ready}"
        );
        // Nor once a lease covers the resumed clock.
        let floor = after.floor;
        renew(&mut after, floor);
        assert!(
            matches!(
                after.receive(&ring, CHANNEL, &text, floor),
                Err(RelayError::Stale | RelayError::BeforeStart)
            ),
            "ready at {ready}"
        );
    }
}

#[test]
fn a_resumed_window_starts_past_the_stored_lease() {
    let lease = NOW + RELAY_LEASE_MICROS;
    let window = ReplayWindow::resume(NOW - 1_000_000, Some(lease));
    assert_eq!(window.floor, startup_floor(lease));
    assert_eq!(window.high_water, window.floor);
    // With no stored lease, or one already behind the readiness reading, the reading decides.
    assert_eq!(ReplayWindow::resume(NOW, None).floor, startup_floor(NOW));
    assert_eq!(
        ReplayWindow::resume(NOW, Some(NOW - RELAY_LEASE_MICROS)).floor,
        startup_floor(NOW)
    );
}

#[test]
fn a_rolling_restart_does_not_push_the_lease_ahead() {
    let ring = ring(&[(1, 0)]);
    // The listeners of one World restart one after another, 5 s apart. The durable lease is
    // stored with `GREATEST(stored, new)`; each listener renews it until it stops.
    let step = 5_000_000;
    let cost = FRESHNESS_MICROS + RELAY_LEASE_MICROS + 1;
    let mut durable: Option<u64> = None;
    let mut running: Option<ReplayWindow> = None;
    let mut db_now = NOW;
    for restart in 0..20_u64 {
        db_now = NOW + restart * step;
        if let Some(mut old) = running.take() {
            let lease = renew(&mut old, db_now);
            durable = durable.max(Some(lease));
        }
        let (window, lease) = resumed(db_now, durable);
        durable = durable.max(Some(lease));
        // The durable lease and the restart cost stay bounded, however many restarts came before.
        assert!(
            durable <= Some(db_now + RELAY_LEASE_MICROS),
            "lease after restart {restart}"
        );
        assert!(
            window.floor <= db_now + cost,
            "floor after restart {restart}"
        );
        running = Some(window);
    }
    // Once the documented cost has passed, the last listener delivers a legitimate line.
    let mut window = running.expect("a listener runs");
    let later = db_now + cost;
    renew(&mut window, later);
    let line = room_line(1, later);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(window.receive(&ring, CHANNEL, &text, later), Ok(line));
}

#[test]
fn debug_never_shows_a_private_recipient() {
    let recipient = recipient(201);
    let destination = RelayDestination::Private { recipient };
    let line = RelayLine {
        destination,
        ..room_line(1, NOW)
    };
    for rendered in [format!("{line:?}"), format!("{destination:?}")] {
        assert!(rendered.contains("Private"));
        assert!(!rendered.contains("201"));
        assert!(!rendered.contains(&format!("{recipient:?}")));
        assert!(!rendered.contains("CharacterId"));
    }
    // A room is not private and stays readable.
    assert!(format!("{:?}", room_line(1, NOW)).contains("English"));
}

#[test]
fn a_received_text_that_chat_text_refuses_is_a_bad_payload() {
    let ring = ring(&[(1, 0)]);
    let valid = room_line(1, NOW);
    let mut prefix = valid.encode().expect("encode");
    prefix.truncate(prefix.len() - 2 - valid.text.as_str().len());
    for text in [
        "a".repeat(256),
        "   ".to_owned(),
        "line\nbreak".to_owned(),
        " padded".to_owned(),
        String::new(),
    ] {
        let mut plaintext = prefix.clone();
        push_prefixed(&mut plaintext, text.as_bytes()).expect("prefixed");
        let sealed = seal_plaintext(&ring, CHANNEL, NOW, plaintext, NONCE).expect("seal");
        assert_eq!(
            open(&ring, CHANNEL, &sealed, NOW),
            Err(RelayError::BadPayload),
            "{text:?}"
        );
    }
}

#[test]
fn a_private_recipient_that_is_not_a_character_id_is_a_bad_payload() {
    let ring = ring(&[(1, 0)]);
    let valid = RelayLine {
        destination: RelayDestination::Private {
            recipient: recipient(9),
        },
        ..room_line(1, NOW)
    };
    let plaintext = valid.encode().expect("encode");
    let start = 1 + ID_BYTES + 8;
    let mut non_v7 = *recipient(9).as_bytes();
    non_v7[6] = 0x40;
    let mut bad_variant = *recipient(9).as_bytes();
    bad_variant[8] = 0xc0;
    for bytes in [[0_u8; ID_BYTES], non_v7, bad_variant] {
        let mut forged = plaintext.clone();
        forged[start..start + ID_BYTES].copy_from_slice(&bytes);
        let sealed = seal_plaintext(&ring, CHANNEL, NOW, forged, NONCE).expect("seal");
        assert_eq!(
            open(&ring, CHANNEL, &sealed, NOW),
            Err(RelayError::BadPayload),
            "{bytes:?}"
        );
    }
}

#[test]
fn a_reconnected_window_drops_what_was_sent_while_it_was_down_and_still_refuses_a_replay() {
    let ring = ring(&[(1, 0)]);
    let (mut window, _) = resumed(NOW - FRESHNESS_MICROS - 1, None);
    let start = window.floor;
    renew(&mut window, start);
    let admitted = room_line(1, start);
    let admitted_text = seal_with_nonce(&ring, CHANNEL, &admitted, NONCE).expect("seal");
    assert_eq!(
        window.receive(&ring, CHANNEL, &admitted_text, start),
        Ok(admitted)
    );
    // The listener drops; a line is sent while it is down, and it reconnects one second later.
    let missed = room_line(2, start + 500_000);
    let missed_text = seal_with_nonce(&ring, CHANNEL, &missed, NONCE).expect("seal");
    let back = start + 1_000_000;
    window.reconnected(back);
    assert_eq!(window.floor, startup_floor(back));
    let floor = window.floor;
    // Until the reading reaches the new floor nothing this node stamps would be admitted.
    assert_eq!(window.admitting_clock(back), None);
    assert_eq!(window.admitting_clock(floor - 1), None);
    renew(&mut window, floor);
    assert_eq!(window.admitting_clock(floor), Some(floor));
    // A copy of the missed line republished after the reconnect is not delivered, nor is the line
    // admitted before the drop; a line sent past the new floor is.
    for (text, at) in [
        (&missed_text, back),
        (&admitted_text, start + FRESHNESS_MICROS),
    ] {
        assert!(
            matches!(
                window.receive(&ring, CHANNEL, text, at),
                Err(RelayError::BeforeStart | RelayError::Stale | RelayError::Replayed)
            ),
            "at {at}"
        );
    }
    let fresh = room_line(3, floor);
    let fresh_text = seal_with_nonce(&ring, CHANNEL, &fresh, NONCE).expect("seal");
    assert_eq!(
        window.receive(&ring, CHANNEL, &fresh_text, floor),
        Ok(fresh)
    );
    // A reconnect reading behind the newest reading moves the floor past that reading, never
    // back, and quick reconnects follow the readings, not the raised clock, so they do not
    // compound.
    window.reconnected(NOW);
    assert_eq!(window.floor, startup_floor(floor));
    window.reconnected(back);
    assert_eq!(window.floor, startup_floor(floor));
}

#[test]
fn after_the_database_clock_steps_back_the_window_admits_what_its_clock_stamps() {
    let ring = ring(&[(1, 0)]);
    let (mut window, _) = resumed(NOW - FRESHNESS_MICROS - 1, None);
    let running = NOW + 10_000_000;
    renew(&mut window, running);
    assert_eq!(window.admitting_clock(running), Some(running));
    // A small step back: the clock stays at the newest reading and a line stamped with it is
    // admitted.
    let stepped = running - 1_000_000;
    let clock = window.admitting_clock(stepped).expect("still covered");
    assert_eq!(clock, running);
    let line = room_line(1, clock);
    let text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(window.receive(&ring, CHANNEL, &text, stepped), Ok(line));
    // A step back past the freshness window: a line stamped with the raw reading would be
    // dropped, so the relay is not ready rather than accepting it.
    let far = NOW - 2 * FRESHNESS_MICROS;
    assert_eq!(window.admitting_clock(far), None);
    let raw = room_line(2, far);
    let raw_text = seal_with_nonce(&ring, CHANNEL, &raw, NONCE).expect("seal");
    assert!(window.receive(&ring, CHANNEL, &raw_text, far).is_err());
}

#[test]
fn a_reconnect_after_the_database_clock_stepped_back_keeps_the_floor_past_the_newest_reading() {
    let ring = ring(&[(1, 0)]);
    let (mut window, _) = resumed(NOW - FRESHNESS_MICROS - 1, None);
    let running = NOW + 10_000_000;
    renew(&mut window, running);
    assert!(window.admitting_clock(running).is_some());
    // While this listener is down another node stamps a line near the newest reading, and the
    // database clock steps back past the freshness window before the reconnect.
    let missed = room_line(1, running + 1_000_000);
    let missed_text = seal_with_nonce(&ring, CHANNEL, &missed, NONCE).expect("seal");
    let back = running - 2 * FRESHNESS_MICROS;
    window.reconnected(back);
    assert_eq!(window.floor, startup_floor(running));
    // A quick second reconnect without a newer reading does not move the floor again.
    window.reconnected(back);
    assert_eq!(window.floor, startup_floor(running));
    // Once the reading and the lease reach the floor the missed line is still refused.
    let floor = window.floor;
    renew(&mut window, floor);
    assert_eq!(window.admitting_clock(floor), Some(floor));
    assert_eq!(
        window.receive(&ring, CHANNEL, &missed_text, floor),
        Err(RelayError::BeforeStart)
    );
}

fn presence(id: u8, sent_at: u64, online: bool) -> PresenceHint {
    PresenceHint {
        message_id: [id; ID_BYTES],
        sent_at_micros: sent_at,
        character: recipient(id),
        online,
    }
}

#[test]
fn a_presence_hint_round_trips_as_kind_three_inside_the_bounds() {
    let ring = ring(&[(1, 0)]);
    for online in [true, false] {
        let hint = presence(3, NOW, online);
        let plaintext = hint.encode();
        assert_eq!(plaintext[0], KIND_PRESENCE);
        assert_eq!(plaintext.len(), PRESENCE_PLAINTEXT_BYTES);
        let text = seal_presence_with_nonce(&ring, CHANNEL, &hint, NONCE).expect("seal");
        assert!(text.len() <= NOTIFY_MAX_BYTES);
        assert_eq!(
            open_relayed(&ring, CHANNEL, &text, NOW),
            Ok(Relayed::Presence(hint))
        );
        // The chat-only opener never hands a hint to chat delivery.
        assert_eq!(
            open(&ring, CHANNEL, &text, NOW),
            Err(RelayError::BadPayload)
        );
    }
    let sealed = seal_presence(
        &ring,
        CHANNEL,
        OutgoingPresence {
            sent_at_micros: NOW,
            character: recipient(4),
            online: true,
        },
    )
    .expect("seal");
    assert!(matches!(
        open_relayed(&ring, CHANNEL, &sealed, NOW),
        Ok(Relayed::Presence(hint)) if hint.online && hint.character == recipient(4)
    ));
}

#[test]
fn a_malformed_presence_hint_is_a_bad_payload() {
    let ring = ring(&[(1, 0)]);
    let good = presence(5, NOW, true).encode();
    let mut bad_flag = good.clone();
    *bad_flag.last_mut().expect("flag") = 2;
    let mut trailing = good.clone();
    trailing.push(0);
    let short = good[..good.len() - 1].to_vec();
    let mut zero_character = good.clone();
    zero_character[1 + ID_BYTES + 8..1 + ID_BYTES + 8 + ID_BYTES].fill(0);
    for plaintext in [bad_flag, trailing, short, zero_character] {
        let text = seal_plaintext(&ring, CHANNEL, NOW, plaintext, NONCE).expect("seal");
        assert_eq!(
            open_relayed(&ring, CHANNEL, &text, NOW),
            Err(RelayError::BadPayload)
        );
    }
    assert_eq!(
        format!("{:?}", presence(5, NOW, true)),
        "PresenceHint { online: true, .. }"
    );
}

#[test]
fn a_presence_hint_shares_the_freshness_and_replay_rules() {
    let ring = ring(&[(1, 0)]);
    let hint = presence(6, NOW, false);
    let text = seal_presence_with_nonce(&ring, CHANNEL, &hint, NONCE).expect("seal");
    let mut window = ReplayWindow::starting_at(NOW);
    assert_eq!(
        window.receive_relayed(&ring, CHANNEL, &text, NOW),
        Ok(Relayed::Presence(hint))
    );
    assert_eq!(
        window.receive_relayed(&ring, CHANNEL, &text, NOW + 1),
        Err(RelayError::Replayed)
    );
    let mut fresh = ReplayWindow::starting_at(NOW);
    assert_eq!(
        fresh.receive_relayed(&ring, CHANNEL, &text, NOW + FRESHNESS_MICROS + 1),
        Err(RelayError::Stale)
    );
    let mut late = ReplayWindow::starting_at(NOW + 1);
    assert_eq!(
        late.receive_relayed(&ring, CHANNEL, &text, NOW + 1),
        Err(RelayError::BeforeStart)
    );
    // A chat line still opens as a line through the shared receiver.
    let line = room_line(7, NOW);
    let line_text = seal_with_nonce(&ring, CHANNEL, &line, NONCE).expect("seal");
    assert_eq!(
        window.receive_relayed(&ring, CHANNEL, &line_text, NOW + 2),
        Ok(Relayed::Line(line))
    );
}
