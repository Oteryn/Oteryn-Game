#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use std::num::NonZeroU64;

const EXTENDED: ItemLookSelection = ItemLookSelection { extended: true };

#[test]
fn request_reuses_existing_item_target_bytes() {
    for value in [1, u64::MAX] {
        let intent = ItemLookIntent {
            target: ItemLookTarget::Item(NonZeroU64::new(value).unwrap()),
        };
        let bytes = encode_item_look_intent_for(intent, EXTENDED).unwrap();
        assert_eq!(bytes, encode_item_target(NonZeroU64::new(value).unwrap()));
        assert_eq!(decode_item_look_intent_for(&bytes, EXTENDED), Ok(intent));
    }
    for bytes in [&[][..], &[8, 0], &[8, 1, 8, 2], &[16, 1], &[8, 0x80]] {
        assert_eq!(
            decode_item_look_intent_for(bytes, EXTENDED),
            Err(ItemLookWireError::Malformed)
        );
    }
    assert_eq!(
        decode_item_look_intent_for(&[0; MAX_ITEM_LOOK_TARGET_BYTES + 1], EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn text_and_fact_free_refusal_round_trip() {
    for result in [
        ItemLookResult::Text("You see cheese.\nCount: 2.\nÉclair".to_owned()),
        ItemLookResult::Refused(ItemLookRefusal::NotAvailable),
        ItemLookResult::Refused(ItemLookRefusal::NotVisible),
        ItemLookResult::Refused(ItemLookRefusal::NotReachable),
        ItemLookResult::Refused(ItemLookRefusal::RateLimited),
    ] {
        let bytes = encode_item_look_result_for(&result, EXTENDED).unwrap();
        assert_eq!(decode_item_look_result_for(&bytes, EXTENDED), Ok(result));
    }
    assert_eq!(
        encode_item_look_result_for(
            &ItemLookResult::Refused(ItemLookRefusal::NotAvailable),
            EXTENDED
        ),
        Ok(vec![18, 0])
    );
    assert_eq!(ITEM_LOOK_CAPABILITY_ID, Some(28));
    assert_eq!(ITEM_LOOK_COMMAND_ID, Some(30));
}

#[test]
fn malformed_results_cannot_carry_hidden_facts() {
    for bytes in [
        &[][..],               // Missing outcome.
        &[10, 0],              // Empty success.
        &[18, 1, b'x'],        // Refusal with facts.
        &[10, 1, 0xff],        // Invalid UTF-8.
        &[10, 2, b'x'],        // Truncated text.
        &[18, 0, 18, 0],       // Repeated refusal.
        &[10, 1, b'x', 18, 0], // Both outcomes.
        &[18, 0, 24, 1],       // Unknown field.
        &[8, 1],               // Wrong wire type.
    ] {
        assert_eq!(
            decode_item_look_result_for(bytes, EXTENDED),
            Err(ItemLookWireError::Malformed)
        );
    }
    assert_eq!(
        encode_item_look_result_for(&ItemLookResult::Text(String::new()), EXTENDED),
        Err(ItemLookWireError::Malformed)
    );
}

#[test]
fn ascii_text_limit_is_512_bytes_on_encode_and_decode() {
    assert_eq!(MAX_ITEM_LOOK_TEXT_BYTES, 512);
    let exact = ItemLookResult::Text("x".repeat(512));
    let encoded = encode_item_look_result_for(&exact, EXTENDED).unwrap();
    assert_eq!(encoded.len(), 515); // Tag + two-byte length + 512 text bytes.
    assert_eq!(decode_item_look_result_for(&encoded, EXTENDED), Ok(exact));
    let oversized = "x".repeat(513);
    assert_eq!(
        encode_item_look_result_for(&ItemLookResult::Text(oversized.clone()), EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, oversized.as_bytes());
    assert_eq!(
        decode_item_look_result_for(&encoded, EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn multibyte_text_limit_counts_utf8_bytes_in_both_directions() {
    let text = "é".repeat(256);
    assert_eq!(text.len(), 512);
    assert_eq!(text.chars().count(), 256);
    let exact = ItemLookResult::Text(text);
    let encoded = encode_item_look_result_for(&exact, EXTENDED).unwrap();
    assert_eq!(decode_item_look_result_for(&encoded, EXTENDED), Ok(exact));
    let oversized = "é".repeat(257);
    assert_eq!(oversized.len(), 514);
    assert_eq!(
        encode_item_look_result_for(&ItemLookResult::Text(oversized.clone()), EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, oversized.as_bytes());
    assert_eq!(
        decode_item_look_result_for(&encoded, EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn text_limit_refuses_old_envelope_sized_text_and_retains_outer_ceiling() {
    // The old envelope-sized result is now refused by the stricter text bound.
    let text = "x".repeat(MAX_COMMAND_RESULT_PAYLOAD_BYTES - 4);
    assert!(text.len() > MAX_ITEM_LOOK_TEXT_BYTES);
    assert_eq!(
        encode_item_look_result_for(&ItemLookResult::Text(text.clone()), EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, text.as_bytes());
    assert_eq!(encoded.len(), MAX_COMMAND_RESULT_PAYLOAD_BYTES);
    assert_eq!(
        decode_item_look_result_for(&encoded, EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    assert_eq!(
        decode_item_look_result_for(&vec![0; MAX_COMMAND_RESULT_PAYLOAD_BYTES + 1], EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn creature_uses_native_spatial_entity_ref_and_full_generation_width() {
    // Same opaque 16-byte identity carrier used by native spatial entity snapshots.
    // No LOOK-specific numeric Creature identifier or kind/name truth is introduced.
    let mut identity = [0_u8; 16];
    identity[12..].copy_from_slice(&7_u32.to_be_bytes());
    for generation in [0, 1, u64::MAX] {
        let entity = EntityRef {
            identity,
            generation,
        };
        let intent = ItemLookIntent {
            target: ItemLookTarget::Creature(entity),
        };
        let encoded = encode_item_look_intent_for(intent, EXTENDED).unwrap();
        assert_eq!(decode_item_look_intent_for(&encoded, EXTENDED), Ok(intent));
        let mut expected_reference = vec![10, 16];
        expected_reference.extend_from_slice(&identity);
        if generation != 0 {
            expected_reference.push(16);
            push_varint(&mut expected_reference, generation);
        }
        let mut expected = Vec::new();
        push_message_field(&mut expected, 2, &expected_reference);
        assert_eq!(encoded, expected);
        assert!(encoded.len() <= MAX_ITEM_LOOK_TARGET_BYTES);
        if generation == u64::MAX {
            assert_eq!(encoded.len(), MAX_ITEM_LOOK_TARGET_BYTES);
        }
    }
}

#[test]
fn creature_target_refuses_malformed_identity_generation_and_trailing_fields() {
    let mut reference = vec![10, 16];
    reference.extend_from_slice(&[7; 16]);
    let mut cases = vec![
        vec![], // Missing required identity.
        vec![10, 0],
        vec![10, 15], // Wrong identity length, truncated.
        vec![16, 1],  // Generation without identity.
        vec![8, 1],   // Wrong identity wire type.
    ];
    for size in [15, 17] {
        let mut bad = vec![10, size];
        bad.extend(std::iter::repeat_n(7, usize::from(size)));
        cases.push(bad);
    }
    for suffix in [
        &[16, 0x80][..], // Truncated generation.
        &[16, 1, 16, 2], // Duplicate generation.
        &[24, 1],        // Unknown field.
        &[18, 0],        // Wrong generation wire type.
    ] {
        let mut bad = reference.clone();
        bad.extend_from_slice(suffix);
        cases.push(bad);
    }
    for bad in cases {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 2, &bad);
        assert_eq!(
            decode_item_look_intent_for(&payload, EXTENDED),
            Err(ItemLookWireError::Malformed)
        );
    }
    let mut valid = Vec::new();
    push_message_field(&mut valid, 2, &reference);
    for suffix in [&[8, 1][..], &[18, 0], &[24, 1], &[0]] {
        let mut payload = valid.clone();
        payload.extend_from_slice(suffix);
        assert_eq!(
            decode_item_look_intent_for(&payload, EXTENDED),
            Err(ItemLookWireError::Malformed)
        );
    }
    let mut duplicate_identity = reference.clone();
    duplicate_identity.extend_from_slice(&reference);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, &duplicate_identity);
    assert_eq!(
        decode_item_look_intent_for(&payload, EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    // Overflow in the final u64 byte is malformed even inside the exact byte ceiling.
    let mut overflow_at_bound = reference.clone();
    overflow_at_bound.push(16);
    overflow_at_bound.extend_from_slice(&[0xff; 9]);
    overflow_at_bound.push(2);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, &overflow_at_bound);
    assert_eq!(payload.len(), MAX_ITEM_LOOK_TARGET_BYTES);
    assert_eq!(
        decode_item_look_intent_for(&payload, EXTENDED),
        Err(ItemLookWireError::Malformed)
    );
    // A longer overflowing u64 also exceeds the bounded native-ref envelope.
    let mut overflow = reference;
    overflow.push(16);
    overflow.extend_from_slice(&[0xff; 10]);
    overflow.push(2);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, &overflow);
    assert_eq!(
        decode_item_look_intent_for(&payload, EXTENDED),
        Err(ItemLookWireError::LimitExceeded)
    );
    // Item-first mixed target still follows the unchanged strict ItemTarget decoder.
    assert_eq!(
        decode_item_look_intent_for(&[8, 1, 18, 0], EXTENDED),
        Err(ItemLookWireError::Malformed)
    );
}

#[test]
fn every_refusal_is_fact_free_strict_oneof_with_exact_proposal_bytes() {
    let refusals = [
        (ItemLookRefusal::NotAvailable, 18),
        (ItemLookRefusal::NotVisible, 26),
        (ItemLookRefusal::NotReachable, 34),
        (ItemLookRefusal::RateLimited, 42),
    ];
    for (refusal, tag) in refusals {
        let result = ItemLookResult::Refused(refusal);
        assert_eq!(
            encode_item_look_result_for(&result, EXTENDED),
            Ok(vec![tag, 0])
        );
        assert_eq!(decode_item_look_result_for(&[tag, 0], EXTENDED), Ok(result));
        for bad in [
            vec![tag, 1, b'x'],
            vec![tag, 0, tag, 0],
            vec![tag, 0, 10, 1, b'x'],
            vec![tag, 0, 48, 0],
        ] {
            assert_eq!(
                decode_item_look_result_for(&bad, EXTENDED),
                Err(ItemLookWireError::Malformed)
            );
        }
        for (_, other) in refusals {
            assert_eq!(
                decode_item_look_result_for(&[tag, 0, other, 0], EXTENDED),
                Err(ItemLookWireError::Malformed)
            );
        }
    }
}

#[test]
fn legacy_selection_preserves_item_bytes_and_refuses_creature_requests() {
    assert_eq!(
        ItemLookSelection::default(),
        ItemLookSelection { extended: false }
    );
    for handle in [1, u64::MAX] {
        let handle = NonZeroU64::new(handle).unwrap();
        let intent = ItemLookIntent {
            target: ItemLookTarget::Item(handle),
        };
        let legacy = encode_item_look_intent(intent).unwrap();
        assert_eq!(legacy, encode_item_target(handle));
        assert_eq!(
            encode_item_look_intent_for(intent, EXTENDED),
            Ok(legacy.clone())
        );
        assert_eq!(decode_item_look_intent(&legacy), Ok(intent));
        assert_eq!(decode_item_look_intent_for(&legacy, EXTENDED), Ok(intent));
    }
    for generation in [0, 1, u64::MAX] {
        let intent = ItemLookIntent {
            target: ItemLookTarget::Creature(EntityRef {
                identity: [7; 16],
                generation,
            }),
        };
        let bytes = encode_item_look_intent_for(intent, EXTENDED).unwrap();
        assert_eq!(
            encode_item_look_intent(intent),
            Err(ItemLookWireError::Malformed)
        );
        assert_eq!(
            decode_item_look_intent(&bytes),
            Err(ItemLookWireError::Malformed)
        );
        assert_eq!(decode_item_look_intent_for(&bytes, EXTENDED), Ok(intent));
    }
}

#[test]
fn server_legacy_refusals_map_to_not_available_and_extended_decode_is_gated() {
    for (refusal, tag) in [
        (ItemLookRefusal::NotAvailable, 18),
        (ItemLookRefusal::NotVisible, 26),
        (ItemLookRefusal::NotReachable, 34),
        (ItemLookRefusal::RateLimited, 42),
    ] {
        let result = ItemLookResult::Refused(refusal);
        assert_eq!(encode_item_look_result(&result), Ok(vec![18, 0]));
        assert_eq!(
            decode_item_look_result(&[18, 0]),
            Ok(ItemLookResult::Refused(ItemLookRefusal::NotAvailable))
        );
        let extended = encode_item_look_result_for(&result, EXTENDED).unwrap();
        assert_eq!(extended, vec![tag, 0]);
        assert_eq!(decode_item_look_result_for(&extended, EXTENDED), Ok(result));
        if refusal != ItemLookRefusal::NotAvailable {
            assert_eq!(
                decode_item_look_result(&extended),
                Err(ItemLookWireError::Malformed)
            );
        }
    }
    let text = ItemLookResult::Text("é".repeat(256));
    let bytes = encode_item_look_result(&text).unwrap();
    assert_eq!(
        encode_item_look_result_for(&text, EXTENDED),
        Ok(bytes.clone())
    );
    assert_eq!(decode_item_look_result(&bytes), Ok(text));
}

#[test]
fn generation_zero_stays_wire_representable_but_is_never_current_admission() {
    let reference = EntityRef {
        identity: [7; 16],
        generation: 0,
    };
    let intent = ItemLookIntent {
        target: ItemLookTarget::Creature(reference),
    };
    let bytes = encode_item_look_intent_for(intent, EXTENDED).unwrap();
    assert_eq!(decode_item_look_intent_for(&bytes, EXTENDED), Ok(intent));
    assert_eq!(
        known_creature_generation(reference),
        Err(ItemLookRefusal::NotVisible)
    );
    for generation in [1, u64::MAX] {
        assert_eq!(
            known_creature_generation(EntityRef {
                generation,
                ..reference
            }),
            NonZeroU64::new(generation).ok_or(ItemLookRefusal::NotVisible)
        );
    }
    // The helper has no current-actor input and cannot substitute a current generation
    // or produce a visibility/name binding for this unknown reference.
}

#[test]
fn negotiated_capability28_has_no_inferred_requirements_and_unknown_ids_are_refused() {
    assert_eq!(ITEM_LOOK_CAPABILITY_ID, Some(28));
    assert_eq!(ITEM_LOOK_COMMAND_ID, Some(30));
    // Actual acceptance validator used by native ServerAccepted/ServerResumeAccepted codecs.
    // DAILY-1 allocated 32; this negative case continues to use an unregistered ID.
    let unregistered = u32::MAX;
    assert_eq!(crate::validate_acceptance_value(1, 1, 1, &[28]), Ok(()));
    assert_eq!(crate::validate_acceptance_value(1, 1, 1, &[]), Ok(()));
    assert_eq!(
        crate::validate_acceptance_value(1, 1, 1, &[unregistered]),
        Err(crate::FoundationProtocolError::CapabilityMismatch)
    );
    assert_eq!(
        crate::validate_acceptance_value(1, 1, 1, &[28, unregistered]),
        Err(crate::FoundationProtocolError::CapabilityMismatch)
    );
    assert_eq!(
        crate::CAPABILITY_REQUIREMENTS_V1
            .iter()
            .find(|entry| entry.0 == 28)
            .map(|entry| entry.1),
        Some(&[][..])
    );
    let registry: serde_json::Value =
        serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON).unwrap();
    let entries: Vec<_> = registry["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["id"] == 28)
        .collect();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry["name"], "ITEM_LOOK_V1");
    assert_eq!(entry["offered"], false);
    assert_eq!(entry["requires"], serde_json::json!([]));
    assert_eq!(entry["command_types"], serde_json::json!([30]));
    assert_eq!(entry["state_domains"], serde_json::json!([]));
    assert_eq!(
        registry["allocation_ranges"]["capability_id"]["next_unallocated"],
        37
    );
    // CP D1268/D1292: command 30 is registered once, gated by 28, with the codec's byte bounds.
    let commands: Vec<_> = registry["command_types"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|command| command["id"] == 30)
        .collect();
    assert_eq!(commands.len(), 1);
    let command = commands[0];
    assert_eq!(command["name"], "ITEM_LOOK_INTENT");
    assert_eq!(command["capability"], 28);
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_ITEM_LOOK_TARGET_BYTES as u64)
    );
    let longest = encode_item_look_result_for(
        &ItemLookResult::Text("a".repeat(MAX_ITEM_LOOK_TEXT_BYTES)),
        EXTENDED,
    )
    .unwrap();
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(longest.len() as u64)
    );
}

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_item_look_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_item_look_intent",
        |rng| ItemLookIntent {
            target: ItemLookTarget::Item(NonZeroU64::new(rng.u64().max(1)).expect("handle")),
        },
        decode_item_look_intent,
        |v| encode_item_look_intent(*v).expect("intent"),
    );
}

#[test]
fn fuzz_decode_item_look_result() {
    crate::decoder_fuzz_support::codec(
        "decode_item_look_result",
        |rng| ItemLookResult::Text("é".repeat((rng.below(256) + 1) as usize)),
        decode_item_look_result,
        |v| encode_item_look_result(v).expect("result"),
    );
}

#[test]
fn fuzz_item_look_selected_codecs() {
    for extended in [false, true] {
        let selection = ItemLookSelection { extended };
        crate::decoder_fuzz_support::codec(
            "decode_item_look_intent_for",
            |rng| ItemLookIntent {
                target: ItemLookTarget::Item(NonZeroU64::new(rng.u64().max(1)).expect("handle")),
            },
            |b| decode_item_look_intent_for(b, selection),
            |v| encode_item_look_intent_for(*v, selection).expect("intent"),
        );
        crate::decoder_fuzz_support::codec(
            "decode_item_look_result_for",
            |rng| ItemLookResult::Text("é".repeat((rng.below(256) + 1) as usize)),
            |b| decode_item_look_result_for(b, selection),
            |v| encode_item_look_result_for(v, selection).expect("result"),
        );
    }
}
