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
fn existing_envelope_ceiling_includes_text_framing() {
    // Field tag + three-byte length at this existing envelope bound.
    let text = "x".repeat(MAX_COMMAND_RESULT_PAYLOAD_BYTES - 4);
    let encoded = encode_item_look_result(&ItemLookResult::Text(text.clone())).unwrap();
    assert_eq!(encoded.len(), MAX_COMMAND_RESULT_PAYLOAD_BYTES);
    assert_eq!(
        decode_item_look_result(&encoded),
        Ok(ItemLookResult::Text(text.clone()))
    );
    assert_eq!(
        encode_item_look_result(&ItemLookResult::Text(format!("{text}x"))),
        Err(ItemLookWireError::LimitExceeded)
    );
    assert_eq!(
        decode_item_look_result(&vec![0; MAX_COMMAND_RESULT_PAYLOAD_BYTES + 1]),
        Err(ItemLookWireError::LimitExceeded)
    );
}
