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
        equipment: Vec::new(),
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
    })
    .expect("main backpack");
    assert_eq!(intent.len(), MAX_ITEM_MOVE_INTENT_BYTES);
    assert_eq!(
        encode_item_target(handle(u64::MAX)).len(),
        MAX_ITEM_TARGET_BYTES
    );
    assert!(
        encode_item_move_result(ItemMoveOutcome::Rejected)
            .expect("WIRE-0 result")
            .len()
            <= 4
    );

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
        equipment: Vec::new(),
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
        let bytes = encode_item_move_intent(&intent).expect("main backpack");
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
        let bytes = encode_item_move_result(outcome).expect("WIRE-0 result");
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
    // The registered bound is the capability 12 worst case; capability 4 alone keeps 13.
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES as u64)
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
            MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES,
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

// ITEM-EQUIP-WIRE-1: capability 12 ITEM_EQUIP_DROP_V1.

fn equipped(slot: EquipmentSlot, entry: ItemEntry) -> EquippedItem {
    EquippedItem { slot, item: entry }
}

/// A full capability 12 view: the backpack, 20 entries and nine equipped items.
fn equipped_inventory(entry: fn(u64) -> ItemEntry) -> CharacterInventory {
    CharacterInventory {
        main_backpack: Some(entry(500)),
        entries: (1..=20).map(entry).collect(),
        equipment: EquipmentSlot::ALL
            .into_iter()
            .zip(600..)
            .map(|(slot, n)| equipped(slot, entry(n)))
            .collect(),
    }
}

fn ground(x: i32, y: i32, floor: i16) -> ItemMoveDestination {
    ItemMoveDestination::Ground(WorldTilePosition { x, y, floor })
}

#[test]
fn without_capability_12_command_9_and_domain_9_decode_as_item_view_1a() {
    // Every ITEM-VIEW-1a payload decodes identically through both entry points.
    let views = [
        CharacterInventory::default(),
        inventory(3, small),
        inventory(29, worst),
    ];
    for view in &views {
        let bytes = encode_character_inventory(view).expect("1a view");
        assert_eq!(
            encode_character_inventory_with_equip_drop(view, false).as_ref(),
            Ok(&bytes)
        );
        assert_eq!(
            decode_character_inventory_with_equip_drop(&bytes, false),
            decode_character_inventory(&bytes)
        );
        // The same bytes mean the same view under capability 12.
        assert_eq!(
            decode_character_inventory_with_equip_drop(&bytes, true),
            Ok(view.clone())
        );
    }
    // Field 3 and the equipment are refused, and the 930-byte bound holds.
    let view = equipped_inventory(small);
    let bytes = encode_character_inventory_with_equip_drop(&view, true).expect("cap 12");
    assert_eq!(
        encode_character_inventory(&view),
        Err(ItemViewWireError::Malformed)
    );
    assert_eq!(
        decode_character_inventory(&bytes),
        Err(ItemViewWireError::Malformed)
    );
    assert_eq!(
        decode_character_inventory(&vec![0; MAX_CHARACTER_INVENTORY_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );

    // Command 9: the new destinations are refused both ways and the 13-byte bound holds.
    for destination in [
        ItemMoveDestination::Equipment(EquipmentSlot::Head),
        ground(1, 2, 7),
    ] {
        let intent = ItemMoveIntent {
            source: handle(5),
            destination,
        };
        assert_eq!(
            encode_item_move_intent(&intent),
            Err(ItemViewWireError::Malformed)
        );
        let bytes = encode_item_move_intent_with_equip_drop(&intent, true).expect("cap 12");
        assert_eq!(
            decode_item_move_intent(&bytes),
            Err(ItemViewWireError::Malformed)
        );
    }
    let backpack = ItemMoveIntent {
        source: handle(u64::MAX),
        destination: ItemMoveDestination::MainBackpack,
    };
    let bytes = encode_item_move_intent(&backpack).expect("1a intent");
    assert_eq!(
        encode_item_move_intent_with_equip_drop(&backpack, true).as_ref(),
        Ok(&bytes)
    );
    assert_eq!(
        decode_item_move_intent_with_equip_drop(&bytes, true),
        Ok(backpack)
    );
    assert_eq!(
        decode_item_move_intent(&[0; MAX_ITEM_MOVE_INTENT_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );

    // Results 10 to 12 are unknown without the capability.
    for outcome in [
        ItemMoveOutcome::SlotMismatch,
        ItemMoveOutcome::RequirementNotMet,
        ItemMoveOutcome::Blocked,
    ] {
        assert_eq!(
            encode_item_move_result(outcome),
            Err(ItemViewWireError::Malformed)
        );
        let bytes = encode_item_move_result_with_equip_drop(outcome, true).expect("cap 12");
        assert_eq!(
            decode_item_move_result(&bytes),
            Err(ItemViewWireError::Malformed)
        );
    }
}

#[test]
fn equipped_views_are_measured_and_bounded_at_30_items() {
    let mut element = Vec::new();
    let mut body = Vec::new();
    push_varint_field(&mut body, 1, EquipmentSlot::Ammo as u64);
    encode_entry(&mut body, 2, &worst(0));
    assert_eq!(body.len(), MAX_EQUIPPED_ITEM_BYTES);
    push_message_field(&mut element, 3, &body);
    assert_eq!(element.len(), MAX_EQUIPPED_ITEM_ELEMENT_BYTES);
    assert_eq!(MAX_EQUIPPED_ITEM_ELEMENT_BYTES, 35);

    let full = equipped_inventory(worst);
    let bytes = encode_character_inventory_with_equip_drop(&full, true).expect("30 items");
    assert_eq!(bytes.len(), MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES);
    assert_eq!(MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES, 966);
    assert_eq!(
        decode_character_inventory_with_equip_drop(&bytes, true),
        Ok(full.clone())
    );
    const { assert!(MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES) };
    const { assert!(MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES <= crate::MAX_SNAPSHOT_CHUNK_BYTES) };

    // Equipment alone, without a backpack, is a view.
    let only = CharacterInventory {
        equipment: vec![equipped(EquipmentSlot::Ring, small(1))],
        ..CharacterInventory::default()
    };
    let bytes = encode_character_inventory_with_equip_drop(&only, true).expect("one slot");
    assert_eq!(
        decode_character_inventory_with_equip_drop(&bytes, true),
        Ok(only)
    );

    // A 31st item is refused both ways.
    let mut over = full;
    over.entries.push(small(21));
    assert_eq!(
        encode_character_inventory_with_equip_drop(&over, true),
        Err(ItemViewWireError::LimitExceeded)
    );
    let mut raw = encode_character_inventory_with_equip_drop(&equipped_inventory(small), true)
        .expect("30 items");
    encode_entry(&mut raw, 2, &small(21));
    assert_eq!(
        decode_character_inventory_with_equip_drop(&raw, true),
        Err(ItemViewWireError::LimitExceeded)
    );
    assert_eq!(
        decode_character_inventory_with_equip_drop(
            &vec![0; MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES + 1],
            true
        ),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn equipped_slots_are_ascending_unique_specified_and_hold_unique_handles() {
    let with = |equipment: Vec<EquippedItem>| CharacterInventory {
        main_backpack: Some(small(500)),
        entries: vec![small(1)],
        equipment,
    };
    let bad_views = [
        // Repeated and unordered slots.
        with(vec![
            equipped(EquipmentSlot::Head, small(2)),
            equipped(EquipmentSlot::Head, small(3)),
        ]),
        with(vec![
            equipped(EquipmentSlot::Legs, small(2)),
            equipped(EquipmentSlot::Head, small(3)),
        ]),
        // A handle shared with an entry, the backpack or another slot.
        with(vec![equipped(EquipmentSlot::Head, small(1))]),
        with(vec![equipped(EquipmentSlot::Head, small(500))]),
        with(vec![
            equipped(EquipmentSlot::Head, small(2)),
            equipped(EquipmentSlot::Feet, small(2)),
        ]),
    ];
    for view in &bad_views {
        assert_eq!(
            encode_character_inventory_with_equip_drop(view, true),
            Err(ItemViewWireError::Malformed),
            "{view:?}"
        );
    }

    let raw = |slot_field: &[u8], item: Option<ItemEntry>| {
        let mut body = slot_field.to_vec();
        if let Some(item) = item {
            encode_entry(&mut body, 2, &item);
        }
        let mut payload = Vec::new();
        push_message_field(&mut payload, 3, &body);
        payload
    };
    let mut doubled = raw(&[0x08, 0x01], Some(small(1)));
    doubled.extend(raw(&[0x08, 0x01], Some(small(2))));
    let mut descending = raw(&[0x08, 0x02], Some(small(1)));
    descending.extend(raw(&[0x08, 0x01], Some(small(2))));
    let cases = [
        raw(&[], Some(small(1))),             // UNSPECIFIED (absent)
        raw(&[0x08, 0x00], Some(small(1))),   // UNSPECIFIED (explicit)
        raw(&[0x08, 0x0a], Some(small(1))),   // unknown slot (the container slot is not one)
        raw(&[0x08, 0x01], None),             // no item
        raw(&[0x08, 0x01, 0x08, 0x02], None), // repeated slot field
        raw(&[0x08, 0x01, 0x18, 0x01], None), // unknown field
        doubled,
        descending,
    ];
    for bad in cases {
        assert_eq!(
            decode_character_inventory_with_equip_drop(&bad, true),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn equip_and_drop_intents_round_trip_and_fail_closed() {
    let mut destinations: Vec<ItemMoveDestination> = EquipmentSlot::ALL
        .into_iter()
        .map(ItemMoveDestination::Equipment)
        .collect();
    destinations.extend([
        ItemMoveDestination::MainBackpack,
        ground(0, 0, 0),
        ground(-1, 1, -1),
        ground(i32::MIN, i32::MAX, i16::MIN),
        ground(i32::MAX, i32::MIN, i16::MAX),
    ]);
    for destination in destinations {
        let intent = ItemMoveIntent {
            source: handle(u64::MAX),
            destination,
        };
        let bytes = encode_item_move_intent_with_equip_drop(&intent, true).expect("cap 12");
        assert!(bytes.len() <= MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES);
        assert_eq!(
            decode_item_move_intent_with_equip_drop(&bytes, true),
            Ok(intent)
        );
    }
    let worst = encode_item_move_intent_with_equip_drop(
        &ItemMoveIntent {
            source: handle(u64::MAX),
            destination: ground(i32::MIN, i32::MIN, i16::MIN),
        },
        true,
    )
    .expect("worst ground");
    assert_eq!(worst.len(), MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES);
    assert_eq!(MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES, 29);
    const { assert!(MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES <= crate::MAX_COMMAND_PAYLOAD_BYTES) };

    let cases: [&[u8]; 11] = [
        &[0x08, 0x05, 0x1a, 0x00],             // equipment UNSPECIFIED (absent)
        &[0x08, 0x05, 0x1a, 0x02, 0x08, 0x00], // equipment UNSPECIFIED (explicit)
        &[0x08, 0x05, 0x1a, 0x02, 0x08, 0x0a], // unknown slot
        &[0x08, 0x05, 0x1a, 0x04, 0x08, 0x01, 0x08, 0x02], // repeated slot field
        &[0x08, 0x05, 0x1a, 0x02, 0x10, 0x01], // a count or other field
        &[0x08, 0x05, 0x22, 0x02, 0x08, 0x01, 0x1a, 0x00], // two destinations
        &[0x08, 0x05, 0x12, 0x00, 0x1a, 0x02, 0x08, 0x01], // backpack and equipment
        &[0x08, 0x05, 0x22, 0x04, 0x08, 0x01, 0x08, 0x02], // repeated x
        &[0x08, 0x05, 0x22, 0x02, 0x20, 0x01], // unknown position field
        &[0x08, 0x05, 0x22, 0x04, 0x18, 0x80, 0x80, 0x04], // floor 32768 out of int16
        &[0x08, 0x05, 0x2a, 0x00],             // an unknown destination
    ];
    for bad in cases {
        assert_eq!(
            decode_item_move_intent_with_equip_drop(bad, true),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
    assert_eq!(
        decode_item_move_intent_with_equip_drop(
            &[0; MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES + 1],
            true
        ),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn equip_and_drop_results_round_trip_within_4_bytes() {
    for outcome in [
        ItemMoveOutcome::Moved,
        ItemMoveOutcome::Rejected,
        ItemMoveOutcome::SlotMismatch,
        ItemMoveOutcome::RequirementNotMet,
        ItemMoveOutcome::Blocked,
    ] {
        let bytes = encode_item_move_result_with_equip_drop(outcome, true).expect("cap 12");
        assert!(bytes.len() <= MAX_ITEM_MOVE_RESULT_BYTES);
        assert_eq!(
            decode_item_move_result_with_equip_drop(&bytes, true),
            Ok(outcome)
        );
    }
    assert_eq!(ItemMoveOutcome::SlotMismatch as u32, 10);
    assert_eq!(ItemMoveOutcome::RequirementNotMet as u32, 11);
    assert_eq!(ItemMoveOutcome::Blocked as u32, 12);
    for bad in [&[0x08, 0x00][..], &[0x08, 0x0d]] {
        assert_eq!(
            decode_item_move_result_with_equip_drop(bad, true),
            Err(ItemViewWireError::Malformed)
        );
    }
}

#[test]
fn wire_slots_map_to_semantic_keys_through_a_separate_table() {
    // GAME-ITEM-01 §6.1 semantic slot keys, without the container slot (the main backpack).
    const SEMANTIC_KEYS: [&str; 9] = [
        "HEAD", "AMULET", "TORSO", "WEAPON", "SHIELD", "LEGS", "FEET", "RING", "EXTRA",
    ];
    assert_eq!(
        EquipmentSlot::ALL.map(|slot| slot as u32),
        [1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
    // The table covers every slot once and every key once; a key is never the wire number.
    let slots: BTreeSet<_> = EQUIPMENT_SLOT_SEMANTIC_KEYS
        .iter()
        .map(|(slot, _)| *slot)
        .collect();
    assert_eq!(slots, EquipmentSlot::ALL.into_iter().collect());
    let keys: BTreeSet<_> = EQUIPMENT_SLOT_SEMANTIC_KEYS
        .iter()
        .map(|(_, key)| *key)
        .collect();
    assert_eq!(keys, SEMANTIC_KEYS.into_iter().collect());
    for slot in EquipmentSlot::ALL {
        let key = slot.semantic_key();
        assert!(SEMANTIC_KEYS.contains(&key));
        assert!(key.parse::<u32>().is_err());
    }
    assert_eq!(EquipmentSlot::Necklace.semantic_key(), "AMULET");
    assert_eq!(EquipmentSlot::RightHand.semantic_key(), "WEAPON");
    assert_eq!(EquipmentSlot::LeftHand.semantic_key(), "SHIELD");
    assert_eq!(EquipmentSlot::Ammo.semantic_key(), "EXTRA");
}

#[test]
fn registry_binds_capability_12() {
    let registry: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let capability = registry["capabilities"]
        .as_array()
        .expect("capabilities")
        .iter()
        .find(|entry| entry["id"] == CAPABILITY_ITEM_EQUIP_DROP_V1)
        .expect("capability 12");
    assert_eq!(capability["name"], "ITEM_EQUIP_DROP_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES)
    );
    assert_eq!(
        CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES,
        [CAPABILITY_ITEM_VIEW_MOVE_V1]
    );
    // It extends capability 4's command type 9 and domain 9 and owns neither.
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(capability["state_domains"], serde_json::json!([]));
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_ITEM_EQUIP_DROP_V1));
}
