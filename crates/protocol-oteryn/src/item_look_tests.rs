#![allow(clippy::unwrap_used)]

use super::*;
use std::num::NonZeroU64;

#[test]
fn request_reuses_existing_item_target_bytes() {
    for value in [1, u64::MAX] {
        let intent = ItemLookIntent {
            target: ItemLookTarget::Item(NonZeroU64::new(value).unwrap()),
        };
        let bytes = encode_item_look_intent(intent);
        assert_eq!(bytes, encode_item_target(NonZeroU64::new(value).unwrap()));
        assert_eq!(decode_item_look_intent(&bytes), Ok(intent));
    }
    for bytes in [&[][..], &[8, 0], &[8, 1, 8, 2], &[16, 1], &[8, 0x80]] {
        assert_eq!(
            decode_item_look_intent(bytes),
            Err(ItemLookWireError::Malformed)
        );
    }
    assert_eq!(
        decode_item_look_intent(&[0; MAX_ITEM_LOOK_TARGET_BYTES + 1]),
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
        let bytes = encode_item_look_result(&result).unwrap();
        assert_eq!(decode_item_look_result(&bytes), Ok(result));
    }
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Refused(ItemLookRefusal::NotAvailable)),
        Ok(vec![18, 0])
    );
    assert_eq!(ITEM_LOOK_CAPABILITY_ID, None);
    assert_eq!(ITEM_LOOK_COMMAND_ID, None);
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
            decode_item_look_result(bytes),
            Err(ItemLookWireError::Malformed)
        );
    }
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Text(String::new())),
        Err(ItemLookWireError::Malformed)
    );
}

#[test]
fn ascii_text_limit_is_512_bytes_on_encode_and_decode() {
    assert_eq!(MAX_ITEM_LOOK_TEXT_BYTES, 512);
    let exact = ItemLookResult::Text("x".repeat(512));
    let encoded = encode_item_look_result(&exact).unwrap();
    assert_eq!(encoded.len(), 515); // Tag + two-byte length + 512 text bytes.
    assert_eq!(decode_item_look_result(&encoded), Ok(exact));
    let oversized = "x".repeat(513);
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Text(oversized.clone())),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, oversized.as_bytes());
    assert_eq!(
        decode_item_look_result(&encoded),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn multibyte_text_limit_counts_utf8_bytes_in_both_directions() {
    let text = "é".repeat(256);
    assert_eq!(text.len(), 512);
    assert_eq!(text.chars().count(), 256);
    let exact = ItemLookResult::Text(text);
    let encoded = encode_item_look_result(&exact).unwrap();
    assert_eq!(decode_item_look_result(&encoded), Ok(exact));
    let oversized = "é".repeat(257);
    assert_eq!(oversized.len(), 514);
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Text(oversized.clone())),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, oversized.as_bytes());
    assert_eq!(
        decode_item_look_result(&encoded),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn text_limit_refuses_old_envelope_sized_text_and_retains_outer_ceiling() {
    // The old envelope-sized result is now refused by the stricter text bound.
    let text = "x".repeat(MAX_COMMAND_RESULT_PAYLOAD_BYTES - 4);
    assert!(text.len() > MAX_ITEM_LOOK_TEXT_BYTES);
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Text(text.clone())),
        Err(ItemLookWireError::LimitExceeded)
    );
    let mut encoded = Vec::new();
    push_message_field(&mut encoded, 1, text.as_bytes());
    assert_eq!(encoded.len(), MAX_COMMAND_RESULT_PAYLOAD_BYTES);
    assert_eq!(
        decode_item_look_result(&encoded),
        Err(ItemLookWireError::LimitExceeded)
    );
    assert_eq!(
        decode_item_look_result(&vec![0; MAX_COMMAND_RESULT_PAYLOAD_BYTES + 1]),
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
        let encoded = encode_item_look_intent(intent);
        assert_eq!(decode_item_look_intent(&encoded), Ok(intent));
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
            decode_item_look_intent(&payload),
            Err(ItemLookWireError::Malformed)
        );
    }
    let mut valid = Vec::new();
    push_message_field(&mut valid, 2, &reference);
    for suffix in [&[8, 1][..], &[18, 0], &[24, 1], &[0]] {
        let mut payload = valid.clone();
        payload.extend_from_slice(suffix);
        assert_eq!(
            decode_item_look_intent(&payload),
            Err(ItemLookWireError::Malformed)
        );
    }
    let mut duplicate_identity = reference.clone();
    duplicate_identity.extend_from_slice(&reference);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, &duplicate_identity);
    assert_eq!(
        decode_item_look_intent(&payload),
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
        decode_item_look_intent(&payload),
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
        decode_item_look_intent(&payload),
        Err(ItemLookWireError::LimitExceeded)
    );
    // Item-first mixed target still follows the unchanged strict ItemTarget decoder.
    assert_eq!(
        decode_item_look_intent(&[8, 1, 18, 0]),
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
        assert_eq!(encode_item_look_result(&result), Ok(vec![tag, 0]));
        assert_eq!(decode_item_look_result(&[tag, 0]), Ok(result));
        for bad in [
            vec![tag, 1, b'x'],
            vec![tag, 0, tag, 0],
            vec![tag, 0, 10, 1, b'x'],
            vec![tag, 0, 48, 0],
        ] {
            assert_eq!(
                decode_item_look_result(&bad),
                Err(ItemLookWireError::Malformed)
            );
        }
        for (_, other) in refusals {
            assert_eq!(
                decode_item_look_result(&[tag, 0, other, 0]),
                Err(ItemLookWireError::Malformed)
            );
        }
    }
}
