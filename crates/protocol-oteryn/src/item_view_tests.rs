#![allow(clippy::expect_used)]

use super::*;
use crate::world_spatial_entities::{CAPABILITY_WORLD_SPATIAL_ENTITIES, MAX_SNAPSHOT_ENTITIES};
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).expect("non-zero")
}

/// The largest entry: every varint at its maximum.
fn worst(n: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(u64::MAX - n),
        item_definition_ref: NonZeroU32::MAX,
        count: NonZeroU32::MAX,
        sub_type: u32::MAX,
    }
}

fn small(n: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(n),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

fn inventory(entries: usize, entry: fn(u64) -> ItemEntry) -> CharacterInventory {
    CharacterInventory {
        main_backpack: Some(entry(500)),
        entries: (1..=entries as u64).map(entry).collect(),
    }
}

fn container(entries: usize, entry: fn(u64) -> ItemEntry) -> OpenContainer {
    OpenContainer {
        container_handle: Some(handle(1_000)),
        entries: (1..=entries as u64).map(entry).collect(),
    }
}

/// Appends raw repeated entries, bypassing the encoder's bound.
fn with_raw_entries(mut payload: Vec<u8>, from: u64, count: usize) -> Vec<u8> {
    for n in 0..count as u64 {
        encode_entry(&mut payload, 2, &small(from + n));
    }
    payload
}

#[test]
fn the_largest_entry_and_payloads_are_measured() {
    let mut element = Vec::new();
    encode_entry(&mut element, 2, &worst(0));
    assert_eq!(element.len(), MAX_ITEM_ENTRY_ELEMENT_BYTES);
    assert_eq!(element[1] as usize, MAX_ITEM_ENTRY_BYTES);

    let full = encode_character_inventory(&inventory(29, worst)).expect("30 items");
    assert_eq!(full.len(), MAX_CHARACTER_INVENTORY_BYTES);
    assert_eq!(MAX_CHARACTER_INVENTORY_BYTES, 930);

    let full = encode_open_container(&OpenContainer {
        container_handle: Some(handle(u64::MAX)),
        entries: (1..=16).map(worst).collect(),
    })
    .expect("16 entries");
    assert_eq!(full.len(), MAX_OPEN_CONTAINER_BYTES);
    assert_eq!(MAX_OPEN_CONTAINER_BYTES, 507);

    let intent = encode_item_move_intent(&ItemMoveIntent {
        source: handle(u64::MAX),
        destination: ItemMoveDestination::MainBackpack,
    });
    assert_eq!(intent.len(), MAX_ITEM_MOVE_INTENT_BYTES);
    assert_eq!(
        encode_item_target(handle(u64::MAX)).len(),
        MAX_ITEM_TARGET_BYTES
    );
    assert!(encode_item_move_result(ItemMoveOutcome::Rejected).len() <= 4);

    // FND-02: every view fits one delta and one snapshot chunk.
    for bound in [MAX_CHARACTER_INVENTORY_BYTES, MAX_OPEN_CONTAINER_BYTES] {
        assert!(bound <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
        assert!(bound <= crate::MAX_SNAPSHOT_CHUNK_BYTES);
    }
    const { assert!(MAX_ITEM_MOVE_INTENT_BYTES <= crate::MAX_COMMAND_PAYLOAD_BYTES) };
}

#[test]
fn character_inventory_round_trips_at_30_items_and_refuses_31_both_ways() {
    for view in [
        CharacterInventory::default(),
        inventory(0, small),
        inventory(3, small),
        inventory(29, worst),
    ] {
        let bytes = encode_character_inventory(&view).expect("encode");
        assert_eq!(decode_character_inventory(&bytes), Ok(view));
    }
    assert_eq!(
        encode_character_inventory(&inventory(30, small)),
        Err(ItemViewWireError::LimitExceeded)
    );
    let raw = with_raw_entries(
        encode_character_inventory(&inventory(29, small)).expect("30 items"),
        100,
        1,
    );
    assert!(raw.len() <= MAX_CHARACTER_INVENTORY_BYTES);
    assert_eq!(
        decode_character_inventory(&raw),
        Err(ItemViewWireError::LimitExceeded)
    );
    assert_eq!(
        decode_character_inventory(&vec![0; MAX_CHARACTER_INVENTORY_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn open_container_round_trips_at_16_entries_and_refuses_17_both_ways() {
    for view in [
        OpenContainer::default(),
        container(0, small),
        container(16, worst),
    ] {
        let bytes = encode_open_container(&view).expect("encode");
        assert_eq!(decode_open_container(&bytes), Ok(view));
    }
    assert!(
        encode_open_container(&OpenContainer::default())
            .expect("empty")
            .is_empty()
    );
    assert_eq!(
        encode_open_container(&container(17, small)),
        Err(ItemViewWireError::LimitExceeded)
    );
    let raw = with_raw_entries(
        encode_open_container(&container(16, small)).expect("16"),
        100,
        1,
    );
    assert_eq!(
        decode_open_container(&raw),
        Err(ItemViewWireError::LimitExceeded)
    );
    assert_eq!(
        decode_open_container(&vec![0; MAX_OPEN_CONTAINER_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn views_refuse_orphan_entries_repeated_handles_and_zero_values() {
    let orphan = CharacterInventory {
        main_backpack: None,
        entries: vec![small(1)],
    };
    assert_eq!(
        encode_character_inventory(&orphan),
        Err(ItemViewWireError::Malformed)
    );
    assert_eq!(
        decode_character_inventory(&with_raw_entries(Vec::new(), 1, 1)),
        Err(ItemViewWireError::Malformed)
    );
    let mut repeated = inventory(2, small);
    repeated.entries[1].handle = repeated.entries[0].handle;
    assert_eq!(
        encode_character_inventory(&repeated),
        Err(ItemViewWireError::Malformed)
    );
    let mut backpack_twice = inventory(1, small);
    backpack_twice.entries[0].handle = handle(500);
    assert_eq!(
        encode_character_inventory(&backpack_twice),
        Err(ItemViewWireError::Malformed)
    );

    let orphan = OpenContainer {
        container_handle: None,
        entries: vec![small(1)],
    };
    assert_eq!(
        encode_open_container(&orphan),
        Err(ItemViewWireError::Malformed)
    );
    let mut self_entry = container(1, small);
    self_entry.entries[0].handle = handle(1_000);
    assert_eq!(
        encode_open_container(&self_entry),
        Err(ItemViewWireError::Malformed)
    );

    // Entry fields: zero handle, definition or count, unknown and repeated fields, truncation.
    let mut valid = Vec::new();
    push_varint_field(&mut valid, 1, 7);
    push_varint_field(&mut valid, 2, 3);
    push_varint_field(&mut valid, 3, 1);
    let wrap = |entry: &[u8]| {
        let mut payload = Vec::new();
        push_varint_field(&mut payload, 1, 1_000);
        push_message_field(&mut payload, 2, entry);
        payload
    };
    assert!(decode_open_container(&wrap(&valid)).is_ok());
    let cases: [&[u8]; 7] = [
        &[0x10, 0x03, 0x18, 0x01],                         // no handle
        &[0x08, 0x07, 0x18, 0x01],                         // no definition
        &[0x08, 0x07, 0x10, 0x03],                         // no count
        &[0x08, 0x00, 0x10, 0x03, 0x18, 0x01],             // zero handle
        &[0x08, 0x07, 0x10, 0x03, 0x18, 0x01, 0x28, 0x01], // unknown field 5
        &[0x08, 0x07, 0x08, 0x08, 0x10, 0x03, 0x18, 0x01], // repeated handle
        &[0x08, 0x07, 0x10, 0x03, 0x18],                   // truncated
    ];
    for bad in cases {
        assert_eq!(
            decode_open_container(&wrap(bad)),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
    // A count above u32 fails closed; an over-long entry is over its bound.
    let wide = [0x08, 0x07, 0x10, 0x03, 0x18, 0x80, 0x80, 0x80, 0x80, 0x10];
    assert_eq!(
        decode_open_container(&wrap(&wide)),
        Err(ItemViewWireError::Malformed)
    );
    assert_eq!(
        decode_open_container(&wrap(&[0; MAX_ITEM_ENTRY_BYTES + 1])),
        Err(ItemViewWireError::LimitExceeded)
    );
    // A zero or repeated container handle and an unknown top-level field.
    assert!(decode_open_container(&[0x08, 0x00]).is_err());
    assert!(decode_open_container(&[0x08, 0x01, 0x08, 0x02]).is_err());
    assert!(decode_open_container(&[0x18, 0x01]).is_err());
    assert!(decode_character_inventory(&[0x18, 0x01]).is_err());
}

#[test]
fn item_move_intent_round_trips_and_fails_closed() {
    for source in [1, u64::MAX] {
        let intent = ItemMoveIntent {
            source: handle(source),
            destination: ItemMoveDestination::MainBackpack,
        };
        let bytes = encode_item_move_intent(&intent);
        assert_eq!(decode_item_move_intent(&bytes), Ok(intent));
    }
    let cases: [&[u8]; 7] = [
        &[],                                   // empty
        &[0x12, 0x00],                         // no source
        &[0x08, 0x05],                         // no destination
        &[0x08, 0x00, 0x12, 0x00],             // zero source
        &[0x08, 0x05, 0x12, 0x02, 0x08, 0x01], // non-empty main_backpack
        &[0x08, 0x05, 0x12, 0x00, 0x12, 0x00], // doubled destination
        &[0x08, 0x05, 0x1a, 0x00],             // a destination not in this slice
    ];
    for bad in cases {
        assert_eq!(
            decode_item_move_intent(bad),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
    assert_eq!(
        decode_item_move_intent(&[0; MAX_ITEM_MOVE_INTENT_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn item_move_results_round_trip_and_refuse_zero_or_unknown() {
    use ItemMoveOutcome as O;
    for outcome in [
        O::Moved,
        O::Stale,
        O::TooFar,
        O::NoBackpack,
        O::NoRoom,
        O::NotOwner,
        O::NotPickupable,
        O::NotSupported,
        O::Rejected,
    ] {
        let bytes = encode_item_move_result(outcome);
        assert!(bytes.len() <= MAX_ITEM_MOVE_RESULT_BYTES);
        assert_eq!(decode_item_move_result(&bytes), Ok(outcome));
    }
    for bad in [&[0x08, 0x00][..], &[0x08, 0x0a], &[], &[0x10, 0x01]] {
        assert_eq!(
            decode_item_move_result(bad),
            Err(ItemViewWireError::Malformed)
        );
    }
    assert_eq!(
        decode_item_move_result(&[0; MAX_ITEM_MOVE_RESULT_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn item_target_round_trips_and_refuses_a_zero_handle() {
    let bytes = encode_item_target(handle(42));
    assert_eq!(decode_item_target(&bytes), Ok(handle(42)));
    for bad in [
        &[][..],
        &[0x08, 0x00],
        &[0x08, 0x01, 0x08, 0x02],
        &[0x10, 0x01],
    ] {
        assert_eq!(decode_item_target(bad), Err(ItemViewWireError::Malformed));
    }
}

#[test]
fn live_handle_bound_is_the_sum_of_the_view_bounds() {
    assert_eq!(
        MAX_LIVE_ITEM_HANDLES,
        (MAX_SNAPSHOT_ENTITIES - 1) + MAX_CHARACTER_INVENTORY_ITEMS + MAX_OPEN_CONTAINER_ENTRIES
    );
}

#[test]
fn registries_bind_the_item_view_ids_and_limits() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let find = |list: &str, id: u32| {
        let matching: Vec<&Value> = protocol[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        matching[0].clone()
    };
    let capability = find("capabilities", CAPABILITY_ITEM_VIEW_MOVE_V1);
    assert_eq!(capability["name"], "ITEM_VIEW_MOVE_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES)
    );
    assert_eq!(
        CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
        &[CAPABILITY_WORLD_SPATIAL_ENTITIES]
    );
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_ITEM_MOVE_INTENT])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([
            STATE_DOMAIN_CHARACTER_INVENTORY,
            STATE_DOMAIN_OPEN_CONTAINER
        ])
    );

    let command = find("command_types", COMMAND_TYPE_ITEM_MOVE_INTENT);
    assert_eq!(command["name"], "ITEM_MOVE_INTENT");
    assert_eq!(command["capability"], CAPABILITY_ITEM_VIEW_MOVE_V1);
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_ITEM_MOVE_INTENT_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_ITEM_MOVE_RESULT_BYTES as u64)
    );

    for (id, name, message, bound, delta, snapshot) in [
        (
            STATE_DOMAIN_CHARACTER_INVENTORY,
            "CHARACTER_INVENTORY",
            "CharacterInventoryV1",
            MAX_CHARACTER_INVENTORY_BYTES,
            DELTA_TYPE_CHARACTER_INVENTORY_V1,
            SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
        ),
        (
            STATE_DOMAIN_OPEN_CONTAINER,
            "OPEN_CONTAINER",
            "OpenContainerV1",
            MAX_OPEN_CONTAINER_BYTES,
            DELTA_TYPE_OPEN_CONTAINER_V1,
            SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
        ),
    ] {
        let domain = find("state_domains", id);
        assert_eq!(domain["name"], name);
        assert_eq!(domain["capability"], CAPABILITY_ITEM_VIEW_MOVE_V1);
        for (key, type_id) in [("delta_types", delta), ("snapshot_types", snapshot)] {
            let types = domain[key].as_array().expect(key);
            assert_eq!(types.len(), 1);
            assert_eq!(types[0]["id"], type_id);
            assert_eq!(types[0]["max_payload_bytes"].as_u64(), Some(bound as u64));
            assert_eq!(
                types[0]["payload_schema"],
                format!("docs/contracts/protocol-oteryn/v1/item_view_v1.proto#{message}")
            );
        }
    }

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |row: &str| {
        resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"] == row)
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    assert_eq!(
        limit("ITEMV0-RL-01"),
        Some(MAX_CHARACTER_INVENTORY_ITEMS as u64)
    );
    assert_eq!(
        limit("ITEMV0-RL-02"),
        Some(MAX_OPEN_CONTAINER_ENTRIES as u64)
    );
    assert_eq!(
        limit("GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX"),
        Some(MAX_OPEN_CONTAINER_ENTRIES as u64)
    );
    assert_eq!(limit("ITEMV0-RL-03"), Some(MAX_LIVE_ITEM_HANDLES as u64));
}
