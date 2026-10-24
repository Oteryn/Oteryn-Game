#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;

const PROTOCOL_REGISTRY: &str = crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON;
const RESOURCE_REGISTRY: &str = crate::RESOURCE_LIMITS_REGISTRY_JSON;
const MALFORMED: PresentationWireError = PresentationWireError::Malformed;
const LIMIT: PresentationWireError = PresentationWireError::LimitExceeded;
const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/presentation_v1.proto";
const SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/presentation_v1.proto");

fn tile(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}

/// The longest position: both coordinates zigzag to five bytes, floor 15.
fn far_tile() -> ActorPosition {
    tile(i32::MIN, i32::MIN, MAX_FLOOR)
}

fn actor(seed: u8, generation: u64) -> EntityRef {
    EntityRef {
        identity: [seed; ENTITY_IDENTITY_BYTES],
        generation,
    }
}

fn event(decision: u32, emission: u32, kind: PresentationEventKind) -> PresentationEvent {
    PresentationEvent {
        decision_ordinal: decision,
        emission_ordinal: emission,
        kind,
    }
}

fn effect(decision: u32, emission: u32) -> PresentationEvent {
    event(
        decision,
        emission,
        PresentationEventKind::MagicEffect {
            position: tile(100, 200, 7),
            effect_id: 1,
            source: PresentationSource::Own,
        },
    )
}

/// The largest event: a value_text with both references and two u64 values.
fn worst_value_text(decision: u32, emission: u32) -> PresentationEvent {
    event(
        decision,
        emission,
        PresentationEventKind::ValueText {
            kind: PresentationValueKind::ExperienceOthers,
            position: far_tile(),
            target: actor(1, u64::MAX),
            attacker: Some(actor(2, u64::MAX)),
            primary: PresentationValue {
                value: u64::MAX,
                color: PresentationColor::White,
            },
            secondary: Some(PresentationValue {
                value: u64::MAX,
                color: PresentationColor::MayaBlue,
            }),
        },
    )
}

fn every_kind() -> Vec<PresentationEvent> {
    vec![
        effect(1, 1),
        event(
            1,
            2,
            PresentationEventKind::Missile {
                from: tile(100, 200, 7),
                to: tile(-3, 0, 0),
                missile_id: MAX_CLIENT_ID,
                source: PresentationSource::Others,
            },
        ),
        event(
            2,
            1,
            PresentationEventKind::SpellWords {
                caster: actor(9, 1),
                position: tile(100, 200, 7),
                spell: u32::MAX,
            },
        ),
        event(
            2,
            2,
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                position: tile(101, 200, 7),
                target: actor(3, 7),
                attacker: None,
                primary: PresentationValue {
                    value: 1,
                    color: PresentationColor::Red,
                },
                secondary: None,
            },
        ),
        event(
            3,
            u32::MAX,
            PresentationEventKind::Sound {
                position: tile(0, 0, 0),
                cue_id: 42,
                source: PresentationSource::Global,
                secondary_cue_id: 0,
            },
        ),
        event(
            u32::MAX,
            1,
            PresentationEventKind::Sound {
                position: tile(5, 5, 1),
                cue_id: 1,
                source: PresentationSource::Creatures,
                secondary_cue_id: MAX_CLIENT_ID,
            },
        ),
    ]
}

#[test]
fn events_round_trip_and_the_largest_event_is_within_its_bound() {
    let events = every_kind();
    let bytes = encode_presentation_batch(&events).expect("encode");
    assert_eq!(decode_presentation_batch(&bytes).expect("decode"), events);

    let worst = encode_presentation_batch(&[worst_value_text(u32::MAX, u32::MAX)]).expect("encode");
    assert_eq!(worst.len(), 126);
    assert!(worst.len() <= MAX_PRESENTATION_EVENT_BYTES);
    assert_eq!(
        decode_presentation_batch(&worst).expect("decode"),
        [worst_value_text(u32::MAX, u32::MAX)]
    );

    // An independent fixture: a magic effect at (1, 2, 7), effect 3, source OWN.
    let fixture = [
        0x0a, 0x12, 0x08, 0x05, 0x10, 0x01, 0x1a, 0x0c, 0x0a, 0x06, 0x08, 0x02, 0x10, 0x04, 0x18,
        0x0e, 0x10, 0x03, 0x18, 0x01,
    ];
    let one = event(
        5,
        1,
        PresentationEventKind::MagicEffect {
            position: tile(1, 2, 7),
            effect_id: 3,
            source: PresentationSource::Own,
        },
    );
    assert_eq!(encode_presentation_batch(&[one]).expect("encode"), fixture);
    assert_eq!(decode_presentation_batch(&fixture).expect("decode"), [one]);

    // The snapshot is empty in both directions.
    assert!(encode_presentation_snapshot().is_empty());
    assert_eq!(decode_presentation_snapshot(&[]), Ok(()));
    assert_eq!(
        decode_presentation_snapshot(&[0x0a, 0x00]),
        Err(PresentationWireError::Malformed)
    );
}

#[test]
fn a_full_batch_round_trips_within_its_bound() {
    let full: Vec<PresentationEvent> = (1..=MAX_PRESENTATION_EVENTS)
        .map(|index| worst_value_text(u32::try_from(index).expect("index"), u32::MAX))
        .collect();
    let bytes = encode_presentation_batch(&full).expect("encode");
    assert!(bytes.len() <= MAX_PRESENTATION_BATCH_BYTES);
    assert_eq!(decode_presentation_batch(&bytes).expect("decode"), full);

    // A 1,025th event is refused in both directions, by count before its bytes.
    let small: Vec<PresentationEvent> = (1..=MAX_PRESENTATION_EVENTS + 1)
        .map(|index| effect(u32::try_from(index).expect("index"), 1))
        .collect();
    assert_eq!(
        encode_presentation_batch(&small),
        Err(PresentationWireError::LimitExceeded)
    );
    let mut over = encode_presentation_batch(&small[..MAX_PRESENTATION_EVENTS]).expect("encode");
    let mut last = Vec::new();
    push_message_field(
        &mut last,
        1,
        &encode_event(&small[MAX_PRESENTATION_EVENTS]).expect("event"),
    );
    over.extend_from_slice(&last);
    assert!(over.len() <= MAX_PRESENTATION_BATCH_BYTES);
    assert_eq!(
        decode_presentation_batch(&over),
        Err(PresentationWireError::LimitExceeded)
    );

    // One byte over the batch bound is refused on length.
    assert_eq!(
        decode_presentation_batch(&vec![0; MAX_PRESENTATION_BATCH_BYTES + 1]),
        Err(PresentationWireError::LimitExceeded)
    );
}

#[test]
fn malformed_events_fail_closed() {
    // Empty, out of order and repeated ordinal pairs.
    assert_eq!(encode_presentation_batch(&[]), Err(MALFORMED));
    assert_eq!(decode_presentation_batch(&[]), Err(MALFORMED));
    assert_eq!(
        encode_presentation_batch(&[effect(2, 1), effect(1, 1)]),
        Err(MALFORMED)
    );
    assert_eq!(
        encode_presentation_batch(&[effect(1, 1), effect(1, 1)]),
        Err(MALFORMED)
    );
    let mut swapped = encode_presentation_batch(&[effect(2, 1)]).expect("encode");
    swapped.extend(encode_presentation_batch(&[effect(1, 2)]).expect("encode"));
    assert_eq!(decode_presentation_batch(&swapped), Err(MALFORMED));

    // Values the encoder refuses as a server fault.
    let bad = [
        effect(0, 1),
        effect(1, 0),
        event(
            1,
            1,
            PresentationEventKind::MagicEffect {
                position: tile(0, 0, 16),
                effect_id: 1,
                source: PresentationSource::Own,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::MagicEffect {
                position: tile(0, 0, -1),
                effect_id: 1,
                source: PresentationSource::Own,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::MagicEffect {
                position: tile(0, 0, 0),
                effect_id: 0,
                source: PresentationSource::Own,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::Missile {
                from: tile(0, 0, 0),
                to: tile(0, 0, 0),
                missile_id: MAX_CLIENT_ID + 1,
                source: PresentationSource::Own,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::SpellWords {
                caster: actor(1, 0),
                position: tile(0, 0, 0),
                spell: 1,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::SpellWords {
                caster: actor(1, 1),
                position: tile(0, 0, 0),
                spell: 0,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::Healed,
                position: tile(0, 0, 0),
                target: actor(1, 1),
                attacker: None,
                primary: PresentationValue {
                    value: 0,
                    color: PresentationColor::PastelRed,
                },
                secondary: None,
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageReceived,
                position: tile(0, 0, 0),
                target: actor(1, 1),
                attacker: Some(actor(2, 0)),
                primary: PresentationValue {
                    value: 1,
                    color: PresentationColor::Red,
                },
                secondary: Some(PresentationValue {
                    value: 0,
                    color: PresentationColor::Purple,
                }),
            },
        ),
        event(
            1,
            1,
            PresentationEventKind::Sound {
                position: tile(0, 0, 0),
                cue_id: 1,
                source: PresentationSource::Own,
                secondary_cue_id: MAX_CLIENT_ID + 1,
            },
        ),
    ];
    for event in bad {
        assert_eq!(
            encode_presentation_batch(&[event]),
            Err(MALFORMED),
            "{event:?}"
        );
    }

    // Wire-level refusals, each one entry of field 1. A valid event body for reference:
    // decision 1, emission 1, magic effect {position {}, effect 1, source OWN}.
    let entry = |body: &[u8]| {
        let mut output = Vec::new();
        push_message_field(&mut output, 1, body);
        output
    };
    assert!(
        decode_presentation_batch(&entry(&[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x06, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x01
        ]))
        .is_ok()
    );
    for body in [
        // An empty oneof.
        &[0x08, 0x01, 0x10, 0x01][..],
        // Two oneof members.
        &[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x06, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x01, 0x1a, 0x06,
            0x0a, 0x00, 0x10, 0x01, 0x18, 0x01,
        ][..],
        // A zero source.
        &[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x06, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x00,
        ][..],
        // An unknown source.
        &[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x06, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x05,
        ][..],
        // A missing source.
        &[0x08, 0x01, 0x10, 0x01, 0x1a, 0x04, 0x0a, 0x00, 0x10, 0x01][..],
        // A missing position.
        &[0x08, 0x01, 0x10, 0x01, 0x1a, 0x04, 0x10, 0x01, 0x18, 0x01][..],
        // A field of another kind (missile `to`) inside a magic effect.
        &[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x08, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x01, 0x12, 0x00,
        ][..],
        // A repeated decision ordinal.
        &[
            0x08, 0x01, 0x08, 0x01, 0x10, 0x01, 0x1a, 0x06, 0x0a, 0x00, 0x10, 0x01, 0x18, 0x01,
        ][..],
        // An unknown event field 8.
        &[0x08, 0x01, 0x10, 0x01, 0x42, 0x00][..],
        // A floor of 16 (zigzag 32).
        &[
            0x08, 0x01, 0x10, 0x01, 0x1a, 0x08, 0x0a, 0x02, 0x18, 0x20, 0x10, 0x01, 0x18, 0x01,
        ][..],
        // A truncated tail.
        &[0x08][..],
    ] {
        assert_eq!(
            decode_presentation_batch(&entry(body)),
            Err(MALFORMED),
            "{body:02x?}"
        );
    }
    // A batch field other than 1.
    assert_eq!(decode_presentation_batch(&[0x12, 0x00]), Err(MALFORMED));

    // A value_text with an unknown kind, a zero colour, or a reference with a short identity.
    let value_text = |kind: u8, color: u8, identity: usize| {
        let mut target = Vec::new();
        push_message_field(&mut target, 1, &vec![1; identity]);
        push_varint_field(&mut target, 2, 1);
        let mut value = Vec::new();
        push_varint_field(&mut value, 1, 5);
        push_varint_field(&mut value, 2, u64::from(color));
        let mut body = Vec::new();
        push_varint_field(&mut body, 1, u64::from(kind));
        push_message_field(&mut body, 2, &[]);
        push_message_field(&mut body, 3, &target);
        push_message_field(&mut body, 5, &value);
        let mut event = vec![0x08, 0x01, 0x10, 0x01];
        push_message_field(&mut event, 6, &body);
        entry(&event)
    };
    assert!(decode_presentation_batch(&value_text(1, 1, ENTITY_IDENTITY_BYTES)).is_ok());
    assert_eq!(
        decode_presentation_batch(&value_text(9, 1, 16)),
        Err(MALFORMED)
    );
    assert_eq!(
        decode_presentation_batch(&value_text(1, 0, 16)),
        Err(MALFORMED)
    );
    assert_eq!(
        decode_presentation_batch(&value_text(1, 14, 16)),
        Err(MALFORMED)
    );
    assert_eq!(
        decode_presentation_batch(&value_text(1, 1, 15)),
        Err(MALFORMED)
    );

    // A 129-byte entry is refused on its length before it is decoded.
    let mut long = vec![0x0a, 0xfe, 0x00];
    long.extend(vec![0; 126]);
    assert_eq!(long.len(), MAX_PRESENTATION_EVENT_BYTES + 1);
    assert_eq!(
        decode_presentation_batch(&long),
        Err(PresentationWireError::LimitExceeded)
    );
}

#[test]
fn overflow_keeps_own_then_nearest_then_ordinals() {
    let candidate = |own, nearness, decision, emission| PresentationCandidate {
        own,
        nearness,
        event: effect(decision, emission),
    };
    // 1,030 candidates: four own events far away, then others by distance.
    let mut candidates = vec![
        candidate(true, 900, 2000, 1),
        candidate(true, 900, 1999, 2),
        candidate(true, 900, 1999, 1),
        candidate(true, 5, 3000, 1),
    ];
    for index in 0..1_026_u32 {
        candidates.push(candidate(false, index.div_ceil(2), 1 + index, 1));
    }
    let kept = select_presentation_batch(candidates.clone());
    assert_eq!(kept.len(), MAX_PRESENTATION_EVENTS);
    let keys: Vec<(u32, u32)> = kept.iter().map(PresentationEvent::order_key).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
    for own in [(2000, 1), (1999, 2), (1999, 1), (3000, 1)] {
        assert!(keys.contains(&own), "{own:?}");
    }
    // The 1,020 nearest others are kept; nearness 510 holds decisions 1020 and 1021, so the tie
    // keeps the lower decision ordinal.
    assert!(keys.contains(&(1, 1)));
    assert!(keys.contains(&(1_020, 1)));
    assert!(!keys.contains(&(1_021, 1)));
    assert!(!keys.contains(&(1_026, 1)));
    // The result does not depend on the input order, and encodes.
    candidates.reverse();
    assert_eq!(select_presentation_batch(candidates), kept);
    assert!(encode_presentation_batch(&kept).is_ok());
}

fn cooldown(kind: ActorCooldownKind, id: u32, expires_at_ms: u64) -> ActorCooldown {
    ActorCooldown {
        kind,
        id,
        expires_at_ms,
    }
}

#[test]
fn cooldowns_round_trip_within_their_bounds() {
    let message = ActorCooldowns {
        server_now_ms: 1_000,
        rtt_ms: Some(40),
        entries: vec![
            cooldown(ActorCooldownKind::Group, 1, 3_000),
            cooldown(ActorCooldownKind::Spell, 12, 2_000),
        ],
    };
    let bytes = encode_actor_cooldowns(&message).expect("encode");
    assert_eq!(
        bytes,
        [
            0x08, 0xe8, 0x07, 0x10, 0x28, 0x18, 0x01, 0x22, 0x07, 0x08, 0x01, 0x10, 0x0c, 0x18,
            0xd0, 0x0f, 0x22, 0x07, 0x08, 0x02, 0x10, 0x01, 0x18, 0xb8, 0x17,
        ]
    );
    // The encoder orders entries by (kind, id); the decoder keeps the wire order.
    let decoded = decode_actor_cooldowns(&bytes).expect("decode");
    assert_eq!(decoded.entries[0], message.entries[1]);
    assert_eq!(decoded.entries[1], message.entries[0]);

    // Before the first ack: no estimate, and an empty message encodes to nothing.
    let provisional = ActorCooldowns {
        server_now_ms: 0,
        rtt_ms: None,
        entries: Vec::new(),
    };
    assert!(
        encode_actor_cooldowns(&provisional)
            .expect("encode")
            .is_empty()
    );
    assert_eq!(decode_actor_cooldowns(&[]).expect("decode"), provisional);
    // A measured round trip of 0 ms is still an estimate.
    let instant = ActorCooldowns {
        rtt_ms: Some(0),
        ..provisional.clone()
    };
    let bytes = encode_actor_cooldowns(&instant).expect("encode");
    assert_eq!(bytes, [0x18, 0x01]);
    assert_eq!(decode_actor_cooldowns(&bytes).expect("decode"), instant);

    // The largest entry is 16 bytes; a full message of 4,096 such entries fits its bound.
    let mut largest = Vec::new();
    push_message_field(
        &mut largest,
        4,
        &[
            0x08, 0x02, 0x10, 0xff, 0xff, 0xff, 0x7f, 0x18, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
        ],
    );
    assert_eq!(largest.len(), 16);
    let full = ActorCooldowns {
        server_now_ms: MAX_CLOCK_MS,
        rtt_ms: Some(u32::MAX),
        entries: (1..=u32::try_from(MAX_COOLDOWN_ENTRIES).expect("entries"))
            .map(|id| {
                if id <= MAX_COOLDOWN_GROUP {
                    cooldown(ActorCooldownKind::Group, id, MAX_CLOCK_MS)
                } else {
                    cooldown(
                        ActorCooldownKind::Spell,
                        MAX_COOLDOWN_SPELL_INDEX - id,
                        MAX_CLOCK_MS,
                    )
                }
            })
            .collect(),
    };
    let bytes = encode_actor_cooldowns(&full).expect("encode");
    assert!(bytes.len() <= MAX_ACTOR_COOLDOWNS_BYTES);
    let decoded = decode_actor_cooldowns(&bytes).expect("decode");
    assert_eq!(decoded.entries.len(), MAX_COOLDOWN_ENTRIES);
    assert_eq!(decoded.server_now_ms, MAX_CLOCK_MS);

    // A 4,097th entry is refused in both directions.
    let mut over = full.clone();
    over.entries.push(cooldown(
        ActorCooldownKind::Spell,
        MAX_COOLDOWN_SPELL_INDEX,
        1,
    ));
    assert_eq!(
        encode_actor_cooldowns(&over),
        Err(PresentationWireError::LimitExceeded)
    );
    let mut bytes = encode_actor_cooldowns(&ActorCooldowns {
        server_now_ms: 0,
        rtt_ms: None,
        entries: full.entries.clone(),
    })
    .expect("encode");
    push_message_field(&mut bytes, 4, &[0x08, 0x01, 0x10, 0x01]);
    assert!(bytes.len() <= MAX_ACTOR_COOLDOWNS_BYTES);
    assert_eq!(
        decode_actor_cooldowns(&bytes),
        Err(PresentationWireError::LimitExceeded)
    );
}

#[test]
fn malformed_cooldowns_fail_closed() {
    let base = ActorCooldowns {
        server_now_ms: 10,
        rtt_ms: Some(1),
        entries: Vec::new(),
    };
    let with = |entries: Vec<ActorCooldown>| ActorCooldowns {
        entries,
        ..base.clone()
    };
    for (message, expected) in [
        (
            with(vec![
                cooldown(ActorCooldownKind::Spell, 1, 5),
                cooldown(ActorCooldownKind::Spell, 1, 6),
            ]),
            Err(MALFORMED),
        ),
        (
            with(vec![cooldown(ActorCooldownKind::Spell, 0, 5)]),
            Err(MALFORMED),
        ),
        (
            with(vec![cooldown(
                ActorCooldownKind::Spell,
                MAX_COOLDOWN_SPELL_INDEX + 1,
                5,
            )]),
            Err(MALFORMED),
        ),
        (
            with(vec![cooldown(
                ActorCooldownKind::Group,
                MAX_COOLDOWN_GROUP + 1,
                5,
            )]),
            Err(MALFORMED),
        ),
        (
            with(vec![cooldown(
                ActorCooldownKind::Group,
                1,
                MAX_CLOCK_MS + 1,
            )]),
            Err(LIMIT),
        ),
        (
            ActorCooldowns {
                server_now_ms: MAX_CLOCK_MS + 1,
                ..base.clone()
            },
            Err(LIMIT),
        ),
    ] {
        assert_eq!(encode_actor_cooldowns(&message), expected, "{message:?}");
    }
    // The same spell id under SPELL and GROUP is two keys.
    assert!(
        encode_actor_cooldowns(&with(vec![
            cooldown(ActorCooldownKind::Spell, 1, 5),
            cooldown(ActorCooldownKind::Group, 1, 5),
        ]))
        .is_ok()
    );

    for bytes in [
        // An unestimated round trip that is not 0.
        &[0x10, 0x05][..],
        &[0x10, 0x05, 0x18, 0x00][..],
        // A bool other than 0 or 1.
        &[0x18, 0x02][..],
        // A repeated key.
        &[
            0x22, 0x04, 0x08, 0x01, 0x10, 0x01, 0x22, 0x04, 0x08, 0x01, 0x10, 0x01,
        ][..],
        // A zero, missing or unknown kind.
        &[0x22, 0x04, 0x08, 0x00, 0x10, 0x01][..],
        &[0x22, 0x02, 0x10, 0x01][..],
        &[0x22, 0x04, 0x08, 0x03, 0x10, 0x01][..],
        // A missing id.
        &[0x22, 0x02, 0x08, 0x01][..],
        // A repeated singular field.
        &[0x08, 0x01, 0x08, 0x01][..],
        // An unknown field.
        &[0x28, 0x01][..],
        &[0x22, 0x06, 0x08, 0x01, 0x10, 0x01, 0x20, 0x01][..],
        // A truncated tail.
        &[0x22, 0x04, 0x08][..],
    ] {
        assert_eq!(
            decode_actor_cooldowns(bytes),
            Err(MALFORMED),
            "{bytes:02x?}"
        );
    }
    // server_now_ms of 2^42.
    assert_eq!(
        decode_actor_cooldowns(&[0x08, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]),
        Err(LIMIT)
    );
    // A 17-byte entry is refused on its length.
    let mut long = vec![0x22, 0x0f];
    long.extend([0; 15]);
    assert_eq!(long.len(), MAX_COOLDOWN_ENTRY_BYTES + 1);
    assert_eq!(decode_actor_cooldowns(&long), Err(LIMIT));
    // The bound counts every byte of an overlong field 4 key: a two-byte key, its length and a
    // 13-byte body fit 16 bytes, and one more body byte (a two-byte kind) does not.
    let body = [
        0x08, 0x01, 0x10, 0xff, 0xff, 0x03, 0x18, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
    ];
    let mut overlong = vec![0xa2, 0x00, 0x0d];
    overlong.extend(body);
    assert_eq!(overlong.len(), MAX_COOLDOWN_ENTRY_BYTES);
    assert_eq!(
        decode_actor_cooldowns(&overlong)
            .expect("decode")
            .entries
            .len(),
        1
    );
    let mut longer = vec![0xa2, 0x00, 0x0e, 0x08, 0x81, 0x00];
    longer.extend(&body[2..]);
    assert_eq!(longer.len(), MAX_COOLDOWN_ENTRY_BYTES + 1);
    assert_eq!(decode_actor_cooldowns(&longer), Err(LIMIT));
    assert_eq!(
        decode_actor_cooldowns(&vec![0; MAX_ACTOR_COOLDOWNS_BYTES + 1]),
        Err(LIMIT)
    );
}

#[test]
fn registry_binds_the_presentation_wire_ids_and_limits() {
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

    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_PRESENTATION_V1));
    assert!(
        crate::CAPABILITY_REQUIREMENTS_V1
            .iter()
            .any(|(capability, requires)| {
                *capability == CAPABILITY_PRESENTATION_V1
                    && *requires == CAPABILITY_PRESENTATION_V1_REQUIRES
            })
    );
    let capability = only("capabilities", CAPABILITY_PRESENTATION_V1);
    assert_eq!(capability["name"], "PRESENTATION_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(capability["requires"], serde_json::json!([6]));
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([
            STATE_DOMAIN_WORLD_PRESENTATION,
            STATE_DOMAIN_ACTOR_COOLDOWNS
        ])
    );

    let types = |domain_id: u32,
                 name: &str,
                 delta: (u32, &str, &str, usize),
                 snapshot: (u32, &str, &str, usize)| {
        let domain = only("state_domains", domain_id);
        assert_eq!(domain["name"], name);
        assert_eq!(domain["capability"], CAPABILITY_PRESENTATION_V1);
        for (section, (id, type_name, message, bytes)) in
            [("delta_types", delta), ("snapshot_types", snapshot)]
        {
            let entries = domain[section].as_array().expect(section);
            assert_eq!(entries.len(), 1, "{section}");
            assert_eq!(entries[0]["id"], id);
            assert_eq!(entries[0]["name"], type_name);
            assert_eq!(
                entries[0]["payload_schema"],
                format!("{SCHEMA_PATH}#{message}").as_str()
            );
            assert_eq!(entries[0]["max_payload_bytes"], bytes);
            assert!(
                SCHEMA.contains(&format!("message {message} {{")),
                "{message}"
            );
        }
    };
    types(
        STATE_DOMAIN_WORLD_PRESENTATION,
        "WORLD_PRESENTATION",
        (
            DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1,
            "WORLD_PRESENTATION_BATCH_V1",
            "WorldPresentationBatchV1",
            MAX_PRESENTATION_BATCH_BYTES,
        ),
        (
            SNAPSHOT_TYPE_WORLD_PRESENTATION_V1,
            "WORLD_PRESENTATION_SNAPSHOT_V1",
            "WorldPresentationSnapshotV1",
            0,
        ),
    );
    types(
        STATE_DOMAIN_ACTOR_COOLDOWNS,
        "ACTOR_COOLDOWNS",
        (
            DELTA_TYPE_ACTOR_COOLDOWNS_V1,
            "ACTOR_COOLDOWNS_DELTA_V1",
            "ActorCooldownsV1",
            MAX_ACTOR_COOLDOWNS_BYTES,
        ),
        (
            SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1,
            "ACTOR_COOLDOWNS_SNAPSHOT_V1",
            "ActorCooldownsV1",
            MAX_ACTOR_COOLDOWNS_BYTES,
        ),
    );

    let limits: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let row = |id: &str| {
        let matches: Vec<&Value> = limits["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{id} registered once");
        matches[0].clone()
    };
    for (id, maximum) in [
        ("SPELLPRES0-RL-01", MAX_PRESENTATION_EVENTS),
        ("SPELLPRES0-RL-02", MAX_PRESENTATION_EVENT_BYTES),
        ("SPELLPRES0-RL-03", MAX_PRESENTATION_BATCH_BYTES),
        ("SPELLPRES0-RL-04", MAX_COOLDOWN_ENTRIES),
        ("SPELLPRES0-RL-05", MAX_COOLDOWN_ENTRY_BYTES),
        ("SPELLPRES0-RL-06", 8),
        ("SPELLPRES0-RL-07", 2),
    ] {
        let entry = row(id);
        assert_eq!(entry["hard_maximum"], maximum, "{id}");
        assert!(
            entry["owner_contract"]
                .as_str()
                .expect("owner contract")
                .ends_with("OTERYN_GAME_SPELL_PRESENT0_SPELL_AND_COMBAT_PRESENTATION_DECISION_2026-09-30.md")
        );
    }
}

#[cfg(test)]
#[path = "presentation_fuzz_tests.rs"]
mod fuzz_tests;
