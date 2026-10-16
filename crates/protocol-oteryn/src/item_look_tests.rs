#![allow(clippy::unwrap_used)]

use super::*;
use std::num::NonZeroU64;

#[test]
fn request_reuses_existing_item_target_bytes() {
    for value in [1, u64::MAX] {
        let intent = ItemLookIntent {
            target: NonZeroU64::new(value).unwrap(),
        };
        let bytes = encode_item_look_intent(intent);
        assert_eq!(bytes, encode_item_target(intent.target));
        assert_eq!(decode_item_look_intent(&bytes), Ok(intent));
    }
    for bytes in [&[][..], &[8, 0], &[8, 1, 8, 2], &[16, 1], &[8, 0x80]] {
        assert_eq!(
            decode_item_look_intent(bytes),
            Err(ItemLookWireError::Malformed)
        );
    }
    assert_eq!(
        decode_item_look_intent(&[0; crate::item_view::MAX_ITEM_TARGET_BYTES + 1]),
        Err(ItemLookWireError::LimitExceeded)
    );
}

#[test]
fn text_and_fact_free_refusal_round_trip() {
    for result in [
        ItemLookResult::Text("You see cheese.\nCount: 2.\nÉclair".to_owned()),
        ItemLookResult::Unavailable,
    ] {
        let bytes = encode_item_look_result(&result).unwrap();
        assert_eq!(decode_item_look_result(&bytes), Ok(result));
    }
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Unavailable),
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
