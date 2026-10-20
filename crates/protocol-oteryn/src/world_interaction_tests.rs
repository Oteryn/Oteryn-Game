#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};
use crate::item_view::{
    EquipmentSlot, ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
    ItemMoveSelection, MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES, WorldTilePosition,
    decode_item_move_intent_for, decode_item_move_intent_with_equip_drop,
    decode_item_move_result_for, encode_item_move_intent_for, encode_item_move_result_for,
};
use crate::world_object::{
    MAX_USE_INTENT_BYTES, MAX_USE_WITH_PLACEMENT_BYTES, MapUseTarget, UseIntent, UseTarget,
    WorldObjectError, WorldObjectTarget, decode_use_intent_target, encode_item_use_intent,
    encode_use_intent, encode_use_item_intent,
};

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto");
const USE_SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_object_v1.proto");
const ITEM_SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/item_view_v1.proto");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

fn push(generation: u64, to_x: i32, to_y: i32) -> PushIntent {
    PushIntent {
        target: EntityRef {
            identity: [0xab; 16],
            generation,
        },
        to_x,
        to_y,
    }
}

#[test]
fn a_push_round_trips_and_the_largest_fits_the_bound() {
    let small = push(1, 32_000, 31_000);
    let bytes = encode_push_intent(&small).expect("encode");
    assert_eq!(decode_push_intent(&bytes), Ok(small));
    // The full sint32 domain round-trips; i32::MIN and i32::MAX are both 5-byte zigzag varints.
    for largest in [
        push(u64::MAX, i32::MIN, i32::MIN),
        push(u64::MAX, i32::MAX, i32::MAX),
        push(u64::MAX, i32::MIN, i32::MAX),
        push(u64::MAX, i32::MAX, i32::MIN),
    ] {
        let bytes = encode_push_intent(&largest).expect("encode");
        assert_eq!(bytes.len(), MAX_PUSH_INTENT_BYTES);
        assert_eq!(decode_push_intent(&bytes), Ok(largest));
    }
    for (to_x, to_y) in [(i32::MIN, 1_000), (-1, i32::MAX), (1 << 20, -(1 << 20) - 1)] {
        let intent = push(1, to_x, to_y);
        let bytes = encode_push_intent(&intent).expect("encode");
        assert!(bytes.len() <= MAX_PUSH_INTENT_BYTES);
        assert_eq!(decode_push_intent(&bytes), Ok(intent));
    }
    // A zero coordinate is written explicitly and round-trips.
    let origin = push(2, 0, 0);
    let bytes = encode_push_intent(&origin).expect("encode");
    assert_eq!(&bytes[bytes.len() - 4..], &[0x18, 0, 0x20, 0]);
    assert_eq!(decode_push_intent(&bytes), Ok(origin));
    assert_eq!(MAX_PUSH_INTENT_BYTES, 41);
}

#[test]
fn malformed_pushes_fail_closed() {
    assert_eq!(
        encode_push_intent(&push(0, 1, 1)),
        Err(WorldInteractionWireError::Malformed)
    );
    let good = encode_push_intent(&push(3, 10, 11)).expect("encode");
    let mut no_identity = Vec::new();
    push_varint_field(&mut no_identity, 2, 3);
    let mut zero_generation = Vec::new();
    push_message_field(&mut zero_generation, 1, &[1; 16]);
    let mut short_identity = Vec::new();
    push_message_field(&mut short_identity, 1, &[1; 15]);
    push_varint_field(&mut short_identity, 2, 1);
    let mut twice_generation = good.clone();
    push_varint_field(&mut twice_generation, 2, 3);
    let mut unknown = good.clone();
    push_varint_field(&mut unknown, 5, 1);
    let mut no_destination = Vec::new();
    push_message_field(&mut no_destination, 1, &[1; 16]);
    push_varint_field(&mut no_destination, 2, 1);
    let mut no_y = no_destination.clone();
    push_varint_field(&mut no_y, 3, 2);
    let mut no_x = no_destination.clone();
    push_varint_field(&mut no_x, 4, 2);
    let truncated = good[..good.len() - 1].to_vec();
    let mut far = Vec::new();
    push_message_field(&mut far, 1, &[1; 16]);
    push_varint_field(&mut far, 2, 1);
    push_varint_field(&mut far, 3, 1 << 32);
    push_varint_field(&mut far, 4, 1);
    let mut oversized = good.clone();
    oversized.extend_from_slice(&[0; MAX_PUSH_INTENT_BYTES]);
    // One byte over the bound fails on length before any field is read.
    let mut one_over = encode_push_intent(&push(u64::MAX, i32::MIN, i32::MAX)).expect("encode");
    assert_eq!(one_over.len(), MAX_PUSH_INTENT_BYTES);
    push_varint(&mut one_over, 0);
    assert_eq!(one_over.len(), MAX_PUSH_INTENT_BYTES + 1);
    for (payload, error) in [
        (Vec::new(), WorldInteractionWireError::Malformed),
        (no_identity, WorldInteractionWireError::Malformed),
        (zero_generation, WorldInteractionWireError::Malformed),
        (short_identity, WorldInteractionWireError::Malformed),
        (twice_generation, WorldInteractionWireError::Malformed),
        (unknown, WorldInteractionWireError::Malformed),
        (no_destination, WorldInteractionWireError::Malformed),
        (no_x, WorldInteractionWireError::Malformed),
        (no_y, WorldInteractionWireError::Malformed),
        (truncated, WorldInteractionWireError::Malformed),
        (far, WorldInteractionWireError::Malformed),
        (oversized, WorldInteractionWireError::LimitExceeded),
        (one_over, WorldInteractionWireError::LimitExceeded),
    ] {
        assert_eq!(decode_push_intent(&payload), Err(error), "{payload:?}");
    }
}

#[test]
fn the_result_carries_the_new_dispositions_and_the_sealed_gate_message_within_four_bytes() {
    for disposition in [
        UseDisposition::Locked,
        UseDisposition::KeyMismatch,
        UseDisposition::PzBlocked,
        UseDisposition::NotMovable,
        UseDisposition::NoRoom,
        UseDisposition::NotPossible,
    ] {
        let bytes = encode_push_result(disposition, None).expect("encode");
        assert!(bytes.len() <= MAX_PUSH_RESULT_BYTES);
        assert_eq!(decode_push_result(&bytes), Ok((disposition, None)));
        assert!(disposition.needs_world_interaction());
        assert!(encode_push_result(disposition, Some(1)).is_err());
    }
    for id in [1, 127] {
        let bytes = encode_push_result(UseDisposition::Sealed, Some(id)).expect("encode");
        assert_eq!(bytes.len(), MAX_PUSH_RESULT_BYTES);
        assert_eq!(
            decode_push_result(&bytes),
            Ok((UseDisposition::Sealed, Some(id)))
        );
    }
    // SEALED needs an id in 1 to 127, in both directions; other dispositions refuse one.
    for id in [None, Some(0), Some(128)] {
        assert!(encode_push_result(UseDisposition::Sealed, id).is_err());
    }
    for payload in [
        &[0x08, 13][..],
        &[0x08, 13, 0x10, 0][..],
        &[0x08, 13, 0x10, 128, 1][..],
        &[0x08, 11, 0x10, 1][..],
        &[0x10, 1][..],
        &[0x08, 13, 0x10, 1, 0x10, 1][..],
        &[0x08, 18][..],
        &[0x08, 19][..],
        &[0x08, 20][..],
    ] {
        assert!(decode_push_result(payload).is_err(), "{payload:?}");
    }
    // RUNE-WIRE-1: the USE-only dispositions 18 and 19 are never a push result.
    for disposition in [
        UseDisposition::TargetIllegal,
        UseDisposition::ProtectionZone,
    ] {
        assert_eq!(
            encode_push_result(disposition, None),
            Err(WorldObjectError::Malformed)
        );
    }
    assert_eq!(
        decode_push_result(&[0x08, 13, 0x10, 1, 0x10]),
        Err(WorldObjectError::LimitExceeded)
    );
    assert!(!UseDisposition::NoTarget.needs_world_interaction());
}

#[test]
fn use_with_map_item_arm_needs_capability_11_and_excludes_the_entity_arm() {
    let handle = ItemHandle::MIN;
    let intent = UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            ItemHandle::new(u64::MAX).expect("handle"),
        )),
        use_with_position: None,
    };
    let bytes = encode_item_use_intent(&intent).expect("encode");
    assert!(bytes.len() <= crate::world_object::MAX_USE_INTENT_BYTES);
    // Capabilities 4, 15 and 11 select the arm; without 11 or without 15 it fails closed.
    assert_eq!(
        decode_use_intent_target(&[4, 15, 11], &bytes),
        Ok(intent.clone())
    );
    assert!(decode_use_intent_target(&[4, 15], &bytes).is_err());
    assert!(decode_use_intent_target(&[4, 11], &bytes).is_err());
    // The entity arm is unchanged and still decodes without capability 11.
    let creature = UseIntent {
        target: UseTarget::Item(handle),
        use_with: Some(EntityRef {
            identity: [1; 16],
            generation: 1,
        }),
        use_with_map_item: None,
        use_with_position: None,
    };
    let creature_bytes = encode_item_use_intent(&creature).expect("encode");
    assert_eq!(
        decode_use_intent_target(&[4, 15], &creature_bytes),
        Ok(creature.clone())
    );
    assert_eq!(
        decode_use_intent_target(&[4, 15, 11], &creature_bytes),
        Ok(creature.clone())
    );
    // The arm alone (no field 2 or 5), with a world object target, twice, mixed with the entity
    // arm, with a zero or missing handle, and as the retired top-level fields 6 and 7.
    let mut arm = Vec::new();
    push_message_field(&mut arm, 3, &[0x08, 0x07]);
    let mut alone = Vec::new();
    push_message_field(&mut alone, 4, &arm);
    let mut with_world_object = Vec::new();
    push_message_field(&mut with_world_object, 1, &[0x0a, 1, b'a']);
    push_message_field(&mut with_world_object, 4, &arm);
    let mut twice = bytes.clone();
    twice.extend_from_slice(&bytes[bytes.len() - 6..]);
    let mut mixed = Vec::new();
    push_message_field(&mut mixed, 3, &[0x08, 0x07]);
    push_message_field(&mut mixed, 1, &[1; 16]);
    push_varint_field(&mut mixed, 2, 1);
    let mut mixed_intent = encode_use_item_intent(handle);
    push_message_field(&mut mixed_intent, 4, &mixed);
    let mut zero_arm = Vec::new();
    push_message_field(&mut zero_arm, 3, &[]);
    let mut zero = encode_use_item_intent(handle);
    push_message_field(&mut zero, 4, &zero_arm);
    let mut zero_value = Vec::new();
    push_message_field(&mut zero_value, 3, &[0x08, 0x00]);
    let mut zero_handle = encode_use_item_intent(handle);
    push_message_field(&mut zero_handle, 4, &zero_value);
    let mut retired_6 = encode_use_item_intent(handle);
    push_message_field(&mut retired_6, 6, &[0x08, 0x07]);
    let mut retired_7 = encode_use_item_intent(handle);
    push_message_field(&mut retired_7, 7, &[0x0a, 1, b'd']);
    for payload in [
        alone,
        with_world_object,
        twice,
        mixed_intent,
        zero,
        zero_handle,
        retired_6,
        retired_7,
    ] {
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
            use_with_map_item: Some(MapUseTarget::Item(handle)),
            use_with_position: None,
        })
        .is_err()
    );
}

fn door(placement_bytes: usize, expected_revision: u64) -> MapUseTarget {
    MapUseTarget::Object(WorldObjectTarget {
        placement: vec![0x5a; placement_bytes],
        expected_revision,
    })
}

fn key_on(target: UseTarget, object: MapUseTarget) -> UseIntent {
    UseIntent {
        target,
        use_with: None,
        use_with_map_item: Some(object),
        use_with_position: None,
    }
}

#[test]
fn use_with_map_object_arm_names_a_handleless_base_object_within_529_bytes() {
    let key = ItemHandle::MIN;
    let hotkey = UseTarget::ItemByDefinition(std::num::NonZeroU32::MIN);
    // A bundle placement (32-byte digest and 8-byte key) at revision 0 and at a later revision,
    // with a handle or a hotkey as the tool, round-trips under capabilities 4, 15 and 11.
    for intent in [
        key_on(UseTarget::Item(key), door(40, 0)),
        key_on(UseTarget::Item(key), door(40, 7)),
        key_on(hotkey.clone(), door(40, 3)),
    ] {
        let bytes = encode_item_use_intent(&intent).expect("encode");
        assert_eq!(decode_use_intent_target(&[4, 15, 11], &bytes), Ok(intent));
        assert!(decode_use_intent_target(&[4, 15], &bytes).is_err());
        assert!(decode_use_intent_target(&[4, 11], &bytes).is_err());
    }
    // The largest: a u64::MAX tool handle, a 496-byte placement and a u64::MAX revision.
    let largest = key_on(
        UseTarget::Item(ItemHandle::new(u64::MAX).expect("handle")),
        door(MAX_USE_WITH_PLACEMENT_BYTES, u64::MAX),
    );
    let bytes = encode_item_use_intent(&largest).expect("encode");
    assert_eq!(bytes.len(), MAX_USE_INTENT_BYTES);
    assert_eq!(decode_use_intent_target(&[4, 15, 11], &bytes), Ok(largest));
    assert_eq!(MAX_USE_WITH_PLACEMENT_BYTES, 496);
    // One byte over the WORLDINT0-RL-22 bound fails closed before any field is read.
    let mut one_over = bytes;
    one_over.push(0);
    assert_eq!(
        decode_use_intent_target(&[4, 15, 11], &one_over),
        Err(WorldObjectError::LimitExceeded)
    );
    // The encoder refuses an empty or over-long placement and a map object with the entity arm.
    assert_eq!(
        encode_item_use_intent(&key_on(UseTarget::Item(key), door(0, 1))),
        Err(WorldObjectError::Malformed)
    );
    assert_eq!(
        encode_item_use_intent(&key_on(
            UseTarget::Item(key),
            door(MAX_USE_WITH_PLACEMENT_BYTES + 1, 1)
        )),
        Err(WorldObjectError::LimitExceeded)
    );
    assert_eq!(
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(key),
            use_with: Some(EntityRef {
                identity: [1; 16],
                generation: 1,
            }),
            use_with_map_item: Some(door(40, 1)),
            use_with_position: None,
        }),
        Err(WorldObjectError::Malformed)
    );
    // The decoder fails closed on the same shapes and on arms combined or repeated.
    let object = |placement: &[u8]| {
        let mut target = Vec::new();
        push_message_field(&mut target, 1, placement);
        push_varint_field(&mut target, 2, 1);
        target
    };
    // A hotkey tool (field 5, index 1) keeps the payload under 529, so the arm's own checks decide.
    let hotkey_with = |use_with: &[u8]| {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 5, &[0x08, 0x01]);
        push_message_field(&mut payload, 4, use_with);
        payload
    };
    let arm = |placement: &[u8]| {
        let mut use_with = Vec::new();
        push_message_field(&mut use_with, 4, &object(placement));
        use_with
    };
    let mut empty_placement = Vec::new();
    push_varint_field(&mut empty_placement, 2, 1);
    let mut empty = Vec::new();
    push_message_field(&mut empty, 4, &empty_placement);
    let long = arm(&[0x5a; MAX_USE_WITH_PLACEMENT_BYTES + 1]);
    let mut both = arm(&[0x5a; 40]);
    push_message_field(&mut both, 3, &[0x08, 0x07]);
    let mut with_entity = arm(&[0x5a; 40]);
    push_message_field(&mut with_entity, 1, &[1; 16]);
    push_varint_field(&mut with_entity, 2, 1);
    let mut twice = arm(&[0x5a; 40]);
    twice.extend_from_slice(&arm(&[0x5a; 40]));
    let mut alone = Vec::new();
    push_message_field(&mut alone, 4, &arm(&[0x5a; 40]));
    let mut on_world_object = encode_use_intent(&WorldObjectTarget {
        placement: vec![0x5a; 40],
        expected_revision: 1,
    })
    .expect("encode");
    push_message_field(&mut on_world_object, 4, &arm(&[0x5a; 40]));
    for (payload, error) in [
        (hotkey_with(&empty), WorldObjectError::Malformed),
        (hotkey_with(&long), WorldObjectError::LimitExceeded),
        (hotkey_with(&both), WorldObjectError::Malformed),
        (hotkey_with(&with_entity), WorldObjectError::Malformed),
        (hotkey_with(&twice), WorldObjectError::Malformed),
        (alone, WorldObjectError::Malformed),
        (on_world_object, WorldObjectError::Malformed),
    ] {
        assert_eq!(
            decode_use_intent_target(&[4, 15, 11], &payload),
            Err(error),
            "{payload:?}"
        );
    }
}

#[test]
fn command_9_takes_the_ground_destination_under_capability_11_without_equipment() {
    let source = ItemHandle::MIN.get();
    let ground = ItemMoveIntent {
        source: ItemHandle::MIN,
        destination: ItemMoveDestination::Ground(WorldTilePosition {
            x: -1_048_576,
            y: 1_048_575,
            floor: -32_768,
        }),
    };
    let equip = ItemMoveIntent {
        source: ItemHandle::MIN,
        destination: ItemMoveDestination::Equipment(EquipmentSlot::Head),
    };
    let world = ItemMoveSelection {
        world_interaction: true,
        ..ItemMoveSelection::default()
    };
    let none = ItemMoveSelection::default();
    let bytes = encode_item_move_intent_for(&ground, world).expect("ground under capability 11");
    assert!(bytes.len() <= MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES);
    assert_eq!(decode_item_move_intent_for(&bytes, world), Ok(ground));
    // Capability 12 alone still admits it; neither capability refuses it.
    assert_eq!(
        decode_item_move_intent_with_equip_drop(&bytes, true),
        Ok(ground)
    );
    assert!(decode_item_move_intent_for(&bytes, none).is_err());
    assert!(encode_item_move_intent_for(&ground, none).is_err());
    // Capability 11 does not admit the equipment destination: that stays with capability 12.
    assert!(encode_item_move_intent_for(&equip, world).is_err());
    let equip_bytes = encode_item_move_intent_for(
        &equip,
        ItemMoveSelection {
            equip_drop: true,
            ..ItemMoveSelection::default()
        },
    )
    .expect("equipment under capability 12");
    assert!(decode_item_move_intent_for(&equip_bytes, world).is_err());
    assert!(source > 0);
    // Results: BLOCKED (the refusal of a ground destination) is admitted, 10 and 11 are not.
    for outcome in [ItemMoveOutcome::Moved, ItemMoveOutcome::Blocked] {
        let bytes = encode_item_move_result_for(outcome, world).expect("result");
        assert_eq!(decode_item_move_result_for(&bytes, world), Ok(outcome));
    }
    for outcome in [
        ItemMoveOutcome::SlotMismatch,
        ItemMoveOutcome::RequirementNotMet,
    ] {
        assert!(encode_item_move_result_for(outcome, world).is_err());
        assert!(encode_item_move_result_for(outcome, none).is_err());
    }
    assert!(encode_item_move_result_for(ItemMoveOutcome::Blocked, none).is_err());
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
    assert!(USE_SCHEMA.contains("uint32 gate_message_id = 2;"));
    assert!(!SCHEMA.contains("command type 9 (item_view_v1.proto), whose"));
    assert!(USE_SCHEMA.contains("UseWithV1 use_with = 4;"));
    assert!(USE_SCHEMA.contains("ItemTargetV1 map_item = 3;"));
    assert!(USE_SCHEMA.contains("WorldObjectTargetV1 map_object = 4;"));
    assert!(USE_SCHEMA.contains("at most 496 bytes"));
    assert!(ITEM_SCHEMA.contains(
        "// Capability 12 or capability 11 (a map-item or Ground source, WORLD-INTERACTION-0 §10.1): the\n  // tile does not accept the item, no line of sight, or a limit is reached.\n  ITEM_MOVE_OUTCOME_BLOCKED = 12;"
    ));
    assert!(
        capability["offer_gate"]
            .as_str()
            .is_some_and(|gate| gate.contains("map_item and map_object arms of USE field 4"))
    );
    assert!(!USE_SCHEMA.contains("use_with_map_item = 6;"));
    assert!(USE_SCHEMA.contains("USE_DISPOSITION_NOT_POSSIBLE = 17;"));
    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |id: &str| {
        let matches: Vec<&Value> = resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{id} registered once");
        matches[0]["hard_maximum"].clone()
    };
    assert_eq!(limit("WORLDINT0-RL-21"), MAX_PUSH_INTENT_BYTES);
    assert_eq!(limit("WORLDINT0-RL-22"), MAX_USE_INTENT_BYTES);
}
