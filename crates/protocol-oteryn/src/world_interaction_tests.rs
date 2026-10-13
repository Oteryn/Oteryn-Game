#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};
use crate::item_view::ItemHandle;
use crate::world_object::{
    UseIntent, UseTarget, decode_use_intent_target, encode_item_use_intent, encode_use_item_intent,
};

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto");
const USE_SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_object_v1.proto");

fn push(generation: u64, to: WorldTilePosition) -> PushIntent {
    PushIntent {
        target: EntityRef {
            identity: [0xab; 16],
            generation,
        },
        to,
    }
}

fn tile(x: i32, y: i32, floor: i16) -> WorldTilePosition {
    WorldTilePosition { x, y, floor }
}

#[test]
fn a_push_round_trips_and_the_largest_fits_the_bound() {
    let small = push(1, tile(32_000, 31_000, 7));
    let bytes = encode_push_intent(&small).expect("encode");
    assert_eq!(decode_push_intent(&bytes), Ok(small));
    let largest = push(u64::MAX, tile(i32::MIN, i32::MIN, i16::MIN));
    let bytes = encode_push_intent(&largest).expect("encode");
    assert_eq!(bytes.len(), MAX_PUSH_INTENT_BYTES);
    assert_eq!(decode_push_intent(&bytes), Ok(largest));
}

#[test]
fn malformed_pushes_fail_closed() {
    assert_eq!(
        encode_push_intent(&push(0, tile(1, 1, 7))),
        Err(WorldInteractionWireError::Malformed)
    );
    let good = encode_push_intent(&push(3, tile(1, 1, 7))).expect("encode");
    let target = &good[..good.iter().position(|b| *b == 0x12).expect("tile")];
    let mut zero_generation = Vec::new();
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, &[1; 16]);
    push_varint_field(&mut inner, 2, 0);
    push_message_field(&mut zero_generation, 1, &inner);
    push_message_field(&mut zero_generation, 2, &[]);
    let mut short_identity = Vec::new();
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, &[1; 15]);
    push_varint_field(&mut inner, 2, 1);
    push_message_field(&mut short_identity, 1, &inner);
    push_message_field(&mut short_identity, 2, &[]);
    let mut twice = good.clone();
    twice.extend_from_slice(&good);
    let mut unknown = good.clone();
    push_varint_field(&mut unknown, 3, 1);
    let mut oversized = good.clone();
    oversized.extend_from_slice(&[0; MAX_PUSH_INTENT_BYTES]);
    for (payload, error) in [
        (target.to_vec(), WorldInteractionWireError::Malformed),
        (Vec::new(), WorldInteractionWireError::Malformed),
        (zero_generation, WorldInteractionWireError::Malformed),
        (short_identity, WorldInteractionWireError::Malformed),
        (twice, WorldInteractionWireError::LimitExceeded),
        (unknown, WorldInteractionWireError::Malformed),
        (oversized, WorldInteractionWireError::LimitExceeded),
    ] {
        assert_eq!(decode_push_intent(&payload), Err(error), "{payload:?}");
    }
}

#[test]
fn the_result_carries_the_new_dispositions_within_four_bytes() {
    for disposition in [
        UseDisposition::Locked,
        UseDisposition::KeyMismatch,
        UseDisposition::Sealed,
        UseDisposition::PzBlocked,
        UseDisposition::NotMovable,
        UseDisposition::NoRoom,
        UseDisposition::NotPossible,
    ] {
        let bytes = encode_push_result(disposition);
        assert!(bytes.len() <= MAX_PUSH_RESULT_BYTES);
        assert_eq!(decode_push_result(&bytes), Ok(disposition));
        assert!(disposition.needs_world_interaction());
    }
    assert!(!UseDisposition::NoTarget.needs_world_interaction());
    assert!(decode_push_result(&[0x08, 18]).is_err());
}

#[test]
fn use_field_6_needs_capability_11_and_excludes_field_4() {
    let handle = ItemHandle::MIN;
    let intent = UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: Some(ItemHandle::new(7).expect("handle")),
    };
    let bytes = encode_item_use_intent(&intent).expect("encode");
    assert!(bytes.len() <= crate::world_object::MAX_USE_INTENT_BYTES);
    // Capabilities 4, 15 and 11 select it; without 11, or without 15, field 6 fails closed.
    assert_eq!(
        decode_use_intent_target(&[4, 15, 11], &bytes),
        Ok(intent.clone())
    );
    assert!(decode_use_intent_target(&[4, 15], &bytes).is_err());
    assert!(decode_use_intent_target(&[4, 11], &bytes).is_err());
    // Field 6 alone, with a world object, twice, with field 4, or with a zero handle.
    let mut alone = Vec::new();
    push_message_field(&mut alone, 6, &[0x08, 0x07]);
    let mut twice = bytes.clone();
    twice.extend_from_slice(&bytes[bytes.len() - 4..]);
    let mut with_creature = bytes.clone();
    let mut creature = Vec::new();
    push_message_field(&mut creature, 1, &[1; 16]);
    push_varint_field(&mut creature, 2, 1);
    push_message_field(&mut with_creature, 4, &creature);
    let mut zero = encode_use_item_intent(handle);
    push_message_field(&mut zero, 6, &[]);
    for payload in [alone, twice, with_creature, zero] {
        assert!(
            decode_use_intent_target(&[4, 15, 11], &payload).is_err(),
            "{payload:?}"
        );
    }
    assert!(
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle),
            use_with: Some(EntityRef {
                identity: [1; 16],
                generation: 1
            }),
            use_with_map_item: Some(handle),
        })
        .is_err()
    );
}

#[test]
fn registry_binds_the_world_interaction_ids_and_limits() {
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
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_WORLD_INTERACTION_V1));
    let capability = only("capabilities", CAPABILITY_WORLD_INTERACTION_V1);
    assert_eq!(capability["name"], "WORLD_INTERACTION_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_WORLD_INTERACTION_V1_REQUIRES)
    );
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_PUSH_INTENT])
    );
    assert_eq!(capability["state_domains"], serde_json::json!([]));
    let command = only("command_types", COMMAND_TYPE_PUSH_INTENT);
    assert_eq!(command["name"], "PUSH_INTENT");
    assert_eq!(command["capability"], CAPABILITY_WORLD_INTERACTION_V1);
    assert_eq!(command["max_payload_bytes"], MAX_PUSH_INTENT_BYTES);
    assert_eq!(command["max_result_payload_bytes"], MAX_PUSH_RESULT_BYTES);
    assert!(SCHEMA.contains("message PushIntentV1 {"));
    assert!(SCHEMA.contains(&format!("At most {MAX_PUSH_INTENT_BYTES} bytes")));
    assert!(USE_SCHEMA.contains("ItemTargetV1 use_with_map_item = 6;"));
    assert!(USE_SCHEMA.contains("USE_DISPOSITION_NOT_POSSIBLE = 17;"));
}
