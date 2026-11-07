//! DONOR-F26-1: synthetic adversarial count/length fixtures, with deterministic work evidence.
#![allow(clippy::expect_used)]

use crate::*;
use std::cell::Cell;
use std::fmt::Debug;

thread_local! {
    static WORK: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

pub(super) fn allocation(count: usize) {
    WORK.with(|work| work.set((work.get().0 + count, work.get().1)));
}
pub(super) fn iteration() {
    WORK.with(|work| work.set((work.get().0, work.get().1 + 1)));
}
fn measured<T>(decode: impl FnOnce() -> T) -> (T, (usize, usize)) {
    WORK.with(|work| work.set((0, 0)));
    let result = decode();
    (result, WORK.with(Cell::get))
}
fn prefix(value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    crate::charm_wire::push_varint(&mut out, value);
    out
}

/// All seven protobuf length readers borrow a checked input slice. The cursor proves that
/// a rejected declaration consumes only its <=10-byte prefix, never its advertised body.
/// MAX_SUPPORTED at this layer is the remaining payload; semantic limits belong to callers.
pub(super) fn borrowed_length_bounds<E: Debug>(
    read: for<'a> fn(&'a [u8], &mut usize) -> Result<&'a [u8], E>,
) {
    for available in [0usize, 1, 16, 128, 256, 4096] {
        for declared in [
            0,
            1,
            available as u64,
            available as u64 + 1,
            u64::from(u8::MAX),
            u64::from(u16::MAX),
            u64::from(u32::MAX),
            u64::MAX / 16 + 1, // multiplication overflow candidate for 16-byte entries
            u64::MAX,          // addition overflow / wire maximum / wrap candidate
        ] {
            let mut input = prefix(declared);
            let start = input.len();
            input.resize(start + available, 0x61);
            let mut cursor = 0;
            let result = read(&input, &mut cursor);
            if declared <= available as u64 {
                let value = result.expect("backed length admitted");
                assert_eq!(value.len(), declared as usize);
                assert_eq!(value.as_ptr(), input[start..].as_ptr());
                assert_eq!(cursor, start + value.len());
            } else {
                assert!(
                    result.is_err(),
                    "declared={declared}, available={available}"
                );
                assert_eq!(cursor, start, "reject before body traversal");
                assert!(cursor <= 10);
            }
        }
    }
    oteryn_test_support::property::for_each_case("f26_borrowed_lengths", |rng, _| {
        let available = (rng.u64() % 257) as usize;
        let declared = if rng.u64() & 1 == 0 {
            rng.u64()
        } else {
            rng.u64() % 258
        };
        let mut input = prefix(declared);
        let start = input.len();
        input.resize(start + available, 0x61);
        let mut cursor = 0;
        let result = read(&input, &mut cursor);
        assert_eq!(result.is_ok(), declared <= available as u64);
        assert_eq!(
            cursor,
            if let Ok(value) = result {
                assert_eq!(value.as_ptr(), input[start..].as_ptr());
                start + value.len()
            } else {
                start
            }
        );
        Ok(())
    });
    for input in [vec![0x80], vec![0xff; 10], vec![0x80; 11]] {
        let mut cursor = 0;
        assert!(read(&input, &mut cursor).is_err());
        assert!(cursor <= input.len().min(10));
    }
}

#[test]
fn f26_foundation_bounded_lengths_and_unknown_fields() {
    for maximum in [0usize, 1, 16, 128, 4096, MAX_WIRE_FRAME_BYTES as usize] {
        for declared in [0, 1, maximum as u64, maximum as u64 + 1, u64::MAX] {
            // Empty backing buffer distinguishes policy rejection from body decoding.
            let input = prefix(declared);
            let mut cursor = 0;
            let result = bounded_length_delimited(
                &input,
                &mut cursor,
                maximum,
                FoundationProtocolError::PayloadLimitExceeded,
            );
            let expected = if declared > maximum as u64 {
                Err(FoundationProtocolError::PayloadLimitExceeded)
            } else if declared != 0 {
                Err(FoundationProtocolError::MalformedEnvelope)
            } else {
                Ok(&[][..])
            };
            assert_eq!(result, expected);
            assert_eq!(cursor, input.len());
        }
        let mut input = prefix(maximum as u64);
        let start = input.len();
        input.resize(start + maximum, 0x61);
        let mut cursor = 0;
        let value = bounded_length_delimited(
            &input,
            &mut cursor,
            maximum,
            FoundationProtocolError::PayloadLimitExceeded,
        )
        .expect("exact supported length");
        assert_eq!(value.len(), maximum);
        assert_eq!(value.as_ptr(), input[start..].as_ptr());
        assert_eq!(cursor, input.len());
    }
    borrowed_length_bounds(unbounded_length_delimited);
    for declared in [0, 1, u64::from(u32::MAX), u64::MAX / 16 + 1, u64::MAX] {
        let input = prefix(declared);
        let mut cursor = 0;
        assert_eq!(skip_field(&input, &mut cursor, 2).is_ok(), declared == 0);
        assert_eq!(cursor, input.len());
    }
}

#[test]
fn f26_frame_length_and_envelope_payload() {
    for count in [
        0,
        1,
        MAX_WIRE_FRAME_BYTES,
        MAX_WIRE_FRAME_BYTES + 1,
        u32::MAX,
    ] {
        let result = FrameLength::from_prefix(&count.to_be_bytes());
        assert_eq!(result.is_ok(), (1..=MAX_WIRE_FRAME_BYTES).contains(&count));
        // No body: an admitted primitive count still cannot admit a truncated frame.
        assert!(decode_framed_envelope(&count.to_be_bytes()).is_err());
    }
    for count in [
        0,
        1,
        2,
        u64::from(MAX_WIRE_FRAME_BYTES),
        u64::from(MAX_WIRE_FRAME_BYTES) + 1,
        u64::from(u32::MAX),
        u64::MAX / 16 + 1,
        u64::MAX,
    ] {
        // LivenessAck with a required nonce; mutate only the envelope's payload length.
        let mut bytes = vec![8, MessageType::LivenessAck as u8, 34];
        bytes.extend(prefix(count));
        bytes.extend([8, 1]);
        assert_eq!(decode_wire_envelope(&bytes).is_ok(), count == 2);
    }
}

#[test]
fn f26_cyclopedia_count_bound_precedes_allocation_and_iteration() {
    for count in [0u8, 1, 8, 9, u8::MAX] {
        for backed in [false, true] {
            let mut bytes = vec![0; 30];
            bytes[1] = 1; // revision
            bytes[28] = 1;
            bytes[29] = count;
            if backed {
                for n in 0..count {
                    bytes.extend([n, 0, 0, 0, 0]);
                }
            }
            let (result, work) = measured(|| cyclopedia::decode_result(&bytes));
            let admitted = count <= 8 && (count == 0 || backed);
            assert_eq!(result.is_ok(), admitted, "count={count}, backed={backed}");
            if let Ok(value) = result {
                assert_eq!(value.visible.len(), usize::from(count));
                assert!(value.visible.capacity() <= 8);
                assert_eq!(work, (usize::from(count), usize::from(count)));
            } else {
                assert_eq!(work, (0, 0), "reject before allocation/loop");
            }
        }
    }
}

#[test]
fn f26_spell_collection_counts_are_bounded_before_work() {
    use spell_presentation_candidate as p;
    for count in [
        0usize,
        1,
        p::MAX_EVENTS,
        p::MAX_EVENTS + 1,
        usize::from(u16::MAX),
    ] {
        let batch = p::Batch {
            content: [1; 32],
            sync_unit: 1,
            events: (0..count.min(p::MAX_EVENTS))
                .map(|n| p::Event {
                    decision_ordinal: n as u64 + 1,
                    emission_ordinal: 0,
                    body: p::Body::Effect {
                        position: world_spatial::ActorPosition {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                        id: 1,
                        source: p::Source::Global,
                    },
                })
                .collect(),
        };
        let mut bytes = p::encode_batch(&batch).expect("bounded batch");
        bytes[41..43].copy_from_slice(&(count as u16).to_le_bytes());
        for backed in [false, true] {
            let input = if backed { &bytes[..] } else { &bytes[..43] };
            let (result, work) = measured(|| p::decode_batch(input));
            assert_eq!(
                result.is_ok(),
                count <= p::MAX_EVENTS && (count == 0 || backed)
            );
            if count > p::MAX_EVENTS {
                assert_eq!(result, Err(p::Error::Limit));
                assert_eq!(work, (0, 0));
            } else {
                assert!(work.0 <= p::MAX_EVENTS && work.1 <= count);
                if let Ok(value) = result {
                    assert_eq!(value.events.len(), count);
                    assert!(value.events.capacity() <= p::MAX_EVENTS);
                    assert_eq!(work, (count, count));
                }
            }
        }
    }
    for count in [
        0usize,
        1,
        p::MAX_COOLDOWNS,
        p::MAX_COOLDOWNS + 1,
        usize::from(u16::MAX),
    ] {
        let value = p::Cooldowns {
            server_now_ms: 0,
            rtt_ms: 0,
            rtt_estimated: false,
            entries: (0..count.min(p::MAX_COOLDOWNS))
                .map(|n| p::Cooldown {
                    kind: p::CooldownKind::Spell,
                    id: n as u32 + 1,
                    expires_at_ms: 1,
                })
                .collect(),
        };
        let mut bytes = p::encode_cooldowns(&value).expect("bounded cooldowns");
        bytes[12..14].copy_from_slice(&(count as u16).to_le_bytes());
        for backed in [false, true] {
            let input = if backed { &bytes[..] } else { &bytes[..14] };
            let (result, work) = measured(|| p::decode_cooldowns(input));
            assert_eq!(
                result.is_ok(),
                count <= p::MAX_COOLDOWNS && (count == 0 || backed)
            );
            if count > p::MAX_COOLDOWNS {
                assert_eq!(result, Err(p::Error::Limit));
                assert_eq!(work, (0, 0));
            } else {
                assert!(work.0 <= p::MAX_COOLDOWNS && work.1 <= count);
                if let Ok(value) = result {
                    assert_eq!(value.entries.len(), count);
                    assert!(value.entries.capacity() <= p::MAX_COOLDOWNS);
                    assert_eq!(work, (count, count));
                }
            }
        }
    }
}

#[test]
fn f26_mail_text_length_bound_before_copy() {
    for length in [0u16, 1, 7996, 7997, u16::MAX] {
        for backed in [false, true] {
            let mut bytes = vec![2];
            bytes.extend(1u64.to_be_bytes());
            bytes.extend(length.to_be_bytes());
            if backed {
                bytes.resize(bytes.len() + usize::from(length), b'a');
            }
            let result = mail::decode_intent(&bytes);
            assert_eq!(result.is_ok(), length <= 7996 && (length == 0 || backed));
            if length > 7996 {
                assert_eq!(result, Err(mail::MailWireError::LimitExceeded));
            }
            if let Ok(mail::MailIntent::WriteText { text, .. }) = result {
                assert_eq!(text.len(), usize::from(length));
                assert!(text.capacity() <= 7996);
            }
        }
    }
}

#[test]
fn f26_capability_and_revision_repetition_limits() {
    // Protobuf repeats do not advertise an element count. Each element is backed by bytes;
    // the count guard must run before accepting the next value or reading its message body.
    for amount in [0usize, 1, MAX_CAPABILITY_COUNT, MAX_CAPABILITY_COUNT + 1] {
        let (mut count, mut previous) = (0, None);
        for n in 0..amount {
            let result = validate_capability(n as u64 + 1, &mut count, &mut previous, false);
            assert_eq!(result.is_ok(), n < MAX_CAPABILITY_COUNT);
        }
        assert_eq!(count, amount);
        assert_eq!(
            previous,
            (amount != 0).then_some(amount.min(MAX_CAPABILITY_COUNT) as u32)
        );
    }
    let (mut count, mut previous) = (usize::MAX, None);
    assert_eq!(
        validate_capability(1, &mut count, &mut previous, false),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );
    assert_eq!(previous, None);

    for maximum in [MAX_COMMAND_EXPECTED_REVISIONS, MAX_STATE_DOMAINS_PER_SYNC] {
        for amount in [0usize, 1, maximum, maximum + 1] {
            let mut count = 0;
            let mut seen = BTreeSet::new();
            for n in 0..amount {
                let mut revision = vec![8];
                revision.extend(prefix(n as u64 + 1));
                let mut input = prefix(revision.len() as u64);
                input.extend(revision);
                let mut cursor = 0;
                let result =
                    validate_revision_list(&input, &mut cursor, &mut count, maximum, &mut seen);
                assert_eq!(result.is_ok(), n < maximum);
                if n >= maximum {
                    assert_eq!(cursor, 0, "guard before reading element length");
                }
            }
            assert_eq!(seen.len(), amount.min(maximum));
        }
        for initial in [maximum, usize::MAX] {
            let mut count = initial;
            let mut cursor = 0;
            let mut seen = BTreeSet::new();
            assert_eq!(
                validate_revision_list(&[], &mut cursor, &mut count, maximum, &mut seen),
                Err(FoundationProtocolError::PayloadLimitExceeded)
            );
            assert_eq!(cursor, 0);
            assert!(seen.is_empty());
        }
    }
}

#[test]
fn f26_snapshot_declarations_do_not_drive_decode_work() {
    for (field, maximum) in [
        (2u8, u64::from(MAX_SNAPSHOT_CHUNKS)),
        (3, MAX_SNAPSHOT_ASSEMBLED_BYTES),
    ] {
        for declared in [0, 1, maximum, maximum + 1, u64::from(u32::MAX), u64::MAX] {
            let mut input = vec![8, 1, field << 3];
            input.extend(prefix(declared));
            let result = decode_snapshot_begin(&input);
            assert_eq!(result.is_ok(), declared <= maximum);
            if declared > maximum {
                assert_eq!(result, Err(FoundationProtocolError::SnapshotLimitExceeded));
            }
            // These are metadata, not counts of an inline body: no payload is required here.
        }
    }
}

pub(super) fn owned_text_bounds<E: Debug>(
    read: fn(&[u8], &mut usize, usize) -> Result<String, E>,
    maxima: &[usize],
    empty_allowed: bool,
) {
    for &maximum in maxima {
        for declared in [
            0,
            1,
            maximum as u64,
            maximum as u64 + 1,
            u64::from(u32::MAX),
            u64::MAX / 16 + 1,
            u64::MAX,
        ] {
            for backed in [false, true] {
                let mut input = prefix(declared);
                let start = input.len();
                // Never construct a huge adversarial body.
                let available = if backed && declared <= maximum as u64 + 1 {
                    declared as usize
                } else {
                    0
                };
                input.resize(start + available, b'a');
                let mut cursor = 0;
                let result = read(&input, &mut cursor, maximum);
                let admitted = declared <= maximum as u64
                    && declared <= available as u64
                    && (declared != 0 || empty_allowed);
                assert_eq!(
                    result.is_ok(),
                    admitted,
                    "maximum={maximum}, declared={declared}"
                );
                if let Ok(value) = result {
                    assert_eq!(value.len(), declared as usize);
                    assert!(value.capacity() <= maximum);
                }
                assert!(cursor <= input.len());
                if declared > available as u64 {
                    assert_eq!(cursor, start);
                }
            }
        }
    }
}
