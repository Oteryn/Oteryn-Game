#![allow(clippy::expect_used)]

use super::*;
use crate::item_view::{
    COMMAND_TYPE_ITEM_MOVE_INTENT, ItemMoveDestination, ItemMoveIntent, ItemMoveSelection,
    MAX_ITEM_MOVE_INTENT_BYTES, MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES, decode_item_move_intent_for,
    decode_item_move_intent_with_equip_drop, encode_item_move_intent_for,
};
use serde_json::Value;
use std::num::NonZeroU32;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).expect("non-zero")
}

/// The largest entry: every varint at its maximum.
fn worst_entry(n: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(u64::MAX - n),
        item_definition_ref: NonZeroU32::MAX,
        count: NonZeroU32::MAX,
        sub_type: u32::MAX,
    }
}

fn small_entry(n: u64) -> ItemEntry {
    ItemEntry {
        handle: handle(n),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

/// The largest view `id`: handles at the top of the range, distinct across views.
fn worst_view(id: u8) -> ContainerView {
    let base = u64::from(id) * 100;
    ContainerView {
        view_id: id,
        container_handle: handle(u64::MAX - base - 98),
        parent_handle: Some(handle(u64::MAX - base - 99)),
        capacity: MAX_CONTAINER_VIEW_ENTRIES as u8,
        entries: (0..MAX_CONTAINER_VIEW_ENTRIES as u64)
            .map(|n| worst_entry(base + n))
            .collect(),
    }
}

fn small_view(id: u8, entries: u64) -> ContainerView {
    let base = 1000 * (u64::from(id) + 1);
    ContainerView {
        view_id: id,
        container_handle: handle(base),
        parent_handle: None,
        capacity: MAX_CONTAINER_VIEW_ENTRIES as u8,
        entries: (1..=entries).map(|n| small_entry(base + n)).collect(),
    }
}

fn views(list: Vec<ContainerView>) -> ContainerViews {
    ContainerViews { views: list }
}

#[test]
fn views_hold_16_views_of_20_entries_and_refuse_more() {
    // One worst-case view is the measured 646 bytes; 16 of them are the registered 10,384.
    let one =
        encode_container_views(&views(vec![worst_view(MAX_CONTAINER_VIEW_ID)])).expect("one view");
    assert_eq!(one.len(), 3 + MAX_CONTAINER_VIEW_BYTES);
    assert_eq!(MAX_CONTAINER_VIEW_BYTES, 646);
    let full = views((0..=MAX_CONTAINER_VIEW_ID).map(worst_view).collect());
    let bytes = encode_container_views(&full).expect("16 views");
    // View 0 omits its proto3 default id, two bytes under the bound.
    assert_eq!(bytes.len(), MAX_CONTAINER_VIEWS_BYTES - 2);
    assert_eq!(MAX_CONTAINER_VIEWS_BYTES, 10_384);
    assert_eq!(decode_container_views(&bytes), Ok(full.clone()));

    // A seventeenth view fails closed both ways (view ids stop at 15, so the extra one repeats
    // an id; the count check comes first).
    let mut over = full.clone();
    over.views.push(small_view(15, 0));
    assert_eq!(
        encode_container_views(&over),
        Err(ItemViewWireError::LimitExceeded)
    );
    let small = views(
        (0..=MAX_CONTAINER_VIEW_ID)
            .map(|id| small_view(id, 0))
            .collect(),
    );
    let mut bytes = encode_container_views(&small).expect("16 small views");
    encode_view(&mut bytes, &small_view(15, 0));
    assert_eq!(
        decode_container_views(&bytes),
        Err(ItemViewWireError::LimitExceeded)
    );
    // A payload one byte over the bound is refused before parsing.
    assert_eq!(
        decode_container_views(&vec![0; MAX_CONTAINER_VIEWS_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );

    // View id 15 is the last; 16 fails closed both ways.
    let mut high = small_view(MAX_CONTAINER_VIEW_ID, 1);
    let bytes = encode_container_views(&views(vec![high.clone()])).expect("view 15");
    assert_eq!(
        decode_container_views(&bytes),
        Ok(views(vec![high.clone()]))
    );
    high.view_id = 16;
    assert_eq!(
        encode_container_views(&views(vec![high.clone()])),
        Err(ItemViewWireError::LimitExceeded)
    );
    let mut body = Vec::new();
    push_varint_field(&mut body, 1, 16);
    push_varint_field(&mut body, 2, 7);
    push_varint_field(&mut body, 4, 1);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &body);
    assert_eq!(
        decode_container_views(&payload),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn a_view_holds_20_entries_within_its_capacity_and_refuses_21() {
    let view = small_view(0, MAX_CONTAINER_VIEW_ENTRIES as u64);
    let bytes = encode_container_views(&views(vec![view.clone()])).expect("20 entries");
    assert_eq!(
        decode_container_views(&bytes),
        Ok(views(vec![view.clone()]))
    );

    // A 21st entry fails closed both ways.
    let mut over = view.clone();
    over.entries.push(small_entry(5000));
    assert_eq!(
        encode_container_views(&views(vec![over])),
        Err(ItemViewWireError::LimitExceeded)
    );
    let mut body = Vec::new();
    push_varint_field(&mut body, 2, view.container_handle.get());
    push_varint_field(&mut body, 4, 20);
    for n in 1..=21 {
        encode_entry(&mut body, 5, &small_entry(n));
    }
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &body);
    assert_eq!(
        decode_container_views(&payload),
        Err(ItemViewWireError::LimitExceeded)
    );

    // Capacity 20 is the last; 21 fails closed. Entries never exceed the capacity, and a zero
    // capacity is malformed.
    let mut wide = small_view(0, 0);
    wide.capacity = 21;
    assert_eq!(
        encode_container_views(&views(vec![wide])),
        Err(ItemViewWireError::LimitExceeded)
    );
    let mut bag = small_view(0, 9);
    bag.capacity = 8;
    assert_eq!(
        encode_container_views(&views(vec![bag.clone()])),
        Err(ItemViewWireError::Malformed)
    );
    bag.entries.pop();
    let bytes = encode_container_views(&views(vec![bag.clone()])).expect("a bag of 8");
    assert_eq!(decode_container_views(&bytes), Ok(views(vec![bag])));
    let mut empty = small_view(0, 0);
    empty.capacity = 0;
    assert_eq!(
        encode_container_views(&views(vec![empty])),
        Err(ItemViewWireError::Malformed)
    );
}

#[test]
fn views_are_ascending_and_hold_unique_handles() {
    // The empty payload is no open view.
    assert_eq!(
        encode_container_views(&ContainerViews::default()),
        Ok(Vec::new())
    );
    assert_eq!(decode_container_views(&[]), Ok(ContainerViews::default()));

    let a = small_view(2, 1);
    let b = small_view(5, 1);
    let valid = views(vec![a.clone(), b.clone()]);
    let bytes = encode_container_views(&valid).expect("two views");
    assert_eq!(decode_container_views(&bytes), Ok(valid));

    let malformed = |list: Vec<ContainerView>| {
        let value = views(list);
        assert_eq!(
            encode_container_views(&value),
            Err(ItemViewWireError::Malformed)
        );
        let mut bytes = Vec::new();
        for view in &value.views {
            encode_view(&mut bytes, view);
        }
        assert_eq!(
            decode_container_views(&bytes),
            Err(ItemViewWireError::Malformed)
        );
    };
    // Out of order, or a repeated id.
    malformed(vec![b.clone(), a.clone()]);
    malformed(vec![a.clone(), a.clone()]);
    // The same container in two views.
    let mut twin = b.clone();
    twin.container_handle = a.container_handle;
    malformed(vec![a.clone(), twin]);
    // The same entry in two views.
    let mut shared = b.clone();
    shared.entries = a.entries.clone();
    malformed(vec![a.clone(), shared]);
    // Within a view: the parent is the container, an entry is the container or the parent, or
    // an entry repeats.
    let mut own_parent = a.clone();
    own_parent.parent_handle = Some(a.container_handle);
    malformed(vec![own_parent]);
    let mut self_entry = a.clone();
    self_entry.entries[0].handle = a.container_handle;
    malformed(vec![self_entry]);
    let mut parent_entry = a.clone();
    parent_entry.parent_handle = Some(a.entries[0].handle);
    malformed(vec![parent_entry]);
    let mut repeated = small_view(2, 2);
    repeated.entries[1].handle = repeated.entries[0].handle;
    malformed(vec![repeated]);
}

#[test]
fn view_fields_fail_closed_when_zero_repeated_or_unknown() {
    let view = |fields: &[(u64, u64)]| {
        let mut body = Vec::new();
        for (field, value) in fields {
            push_varint_field(&mut body, *field, *value);
        }
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &body);
        decode_container_views(&payload)
    };
    // A minimal view: id 0, container 7, capacity 1; the parent is optional.
    assert!(view(&[(2, 7), (4, 1)]).is_ok());
    assert!(view(&[(2, 7), (3, 8), (4, 1)]).is_ok());
    // No container, a zero container, a zero or absent capacity.
    for fields in [
        &[(4, 1)][..],
        &[(2, 0), (4, 1)],
        &[(2, 7), (4, 0)],
        &[(2, 7)],
        // A repeated field and an unknown field.
        &[(2, 7), (2, 8), (4, 1)],
        &[(2, 7), (4, 1), (6, 1)],
    ] {
        assert_eq!(
            view(fields),
            Err(ItemViewWireError::Malformed),
            "{fields:?}"
        );
    }
    // An entry with a zero item definition fails closed inside a view.
    let mut body = Vec::new();
    push_varint_field(&mut body, 2, 7);
    push_varint_field(&mut body, 4, 1);
    let mut entry = Vec::new();
    push_varint_field(&mut entry, 1, 9);
    push_varint_field(&mut entry, 3, 1);
    push_message_field(&mut body, 5, &entry);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &body);
    assert_eq!(
        decode_container_views(&payload),
        Err(ItemViewWireError::Malformed)
    );
    // An unknown top-level field.
    assert_eq!(
        decode_container_views(&[0x10, 0x01]),
        Err(ItemViewWireError::Malformed)
    );
}

#[test]
fn view_intents_round_trip_within_15_bytes_and_fail_closed() {
    let worst = ContainerViewIntent::Open {
        handle: handle(u64::MAX),
        replace_view: Some(MAX_CONTAINER_VIEW_ID),
    };
    for intent in [
        worst,
        ContainerViewIntent::Open {
            handle: handle(1),
            replace_view: None,
        },
        // Proto3 `optional`: a replace of view 0 is present on the wire.
        ContainerViewIntent::Open {
            handle: handle(1),
            replace_view: Some(0),
        },
        ContainerViewIntent::Close { view_id: 0 },
        ContainerViewIntent::Close {
            view_id: MAX_CONTAINER_VIEW_ID,
        },
        ContainerViewIntent::Up { view_id: 0 },
        ContainerViewIntent::Up {
            view_id: MAX_CONTAINER_VIEW_ID,
        },
    ] {
        let bytes = encode_container_view_intent(&intent).expect("intent");
        assert!(bytes.len() <= MAX_CONTAINER_VIEW_INTENT_BYTES, "{intent:?}");
        assert_eq!(decode_container_view_intent(&bytes), Ok(intent));
    }
    assert_eq!(
        encode_container_view_intent(&worst).expect("worst").len(),
        MAX_CONTAINER_VIEW_INTENT_BYTES
    );
    assert_eq!(MAX_CONTAINER_VIEW_INTENT_BYTES, 15);
    assert_eq!(
        encode_container_view_intent(&ContainerViewIntent::Close { view_id: 3 }),
        Ok(vec![0x12, 0x02, 0x08, 0x03])
    );
    assert_eq!(
        encode_container_view_intent(&ContainerViewIntent::Open {
            handle: handle(1),
            replace_view: Some(0),
        }),
        Ok(vec![0x0a, 0x04, 0x08, 0x01, 0x10, 0x00])
    );

    // View 16 fails closed both ways.
    for intent in [
        ContainerViewIntent::Open {
            handle: handle(1),
            replace_view: Some(16),
        },
        ContainerViewIntent::Close { view_id: 16 },
        ContainerViewIntent::Up { view_id: 16 },
    ] {
        assert_eq!(
            encode_container_view_intent(&intent),
            Err(ItemViewWireError::LimitExceeded)
        );
    }
    for bad in [
        &[0x0a, 0x04, 0x08, 0x01, 0x10, 0x10][..],
        &[0x12, 0x02, 0x08, 0x10],
        &[0x1a, 0x02, 0x08, 0x10],
    ] {
        assert_eq!(
            decode_container_view_intent(bad),
            Err(ItemViewWireError::LimitExceeded)
        );
    }
    // Over 15 bytes.
    assert_eq!(
        decode_container_view_intent(&[0; MAX_CONTAINER_VIEW_INTENT_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
    for bad in [
        // No action, or two.
        &[][..],
        &[0x12, 0x00, 0x1a, 0x00],
        &[0x12, 0x00, 0x12, 0x00],
        // An open without a handle, with a zero handle, or with a repeated handle.
        &[0x0a, 0x00],
        &[0x0a, 0x02, 0x08, 0x00],
        &[0x0a, 0x04, 0x08, 0x01, 0x08, 0x02],
        // A repeated replace_view, an unknown field in a body, an unknown action.
        &[0x0a, 0x06, 0x08, 0x01, 0x10, 0x01, 0x10, 0x02],
        &[0x12, 0x02, 0x10, 0x01],
        &[0x22, 0x00],
        // A truncated body.
        &[0x0a, 0x04, 0x08, 0x01],
    ] {
        assert_eq!(
            decode_container_view_intent(bad),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn view_results_round_trip_within_4_bytes_and_refuse_zero_or_unknown() {
    for (outcome, value) in [
        (ContainerViewOutcome::Opened, 1),
        (ContainerViewOutcome::Closed, 2),
        (ContainerViewOutcome::Stale, 3),
        (ContainerViewOutcome::TooFar, 4),
        (ContainerViewOutcome::NotAContainer, 5),
        (ContainerViewOutcome::TooManyViews, 6),
    ] {
        assert_eq!(outcome as u32, value);
        let bytes = encode_container_view_result(outcome);
        assert!(bytes.len() <= MAX_CONTAINER_VIEW_RESULT_BYTES);
        assert_eq!(decode_container_view_result(&bytes), Ok(outcome));
    }
    for bad in [&[][..], &[0x08, 0x00], &[0x08, 0x07], &[0x10, 0x01]] {
        assert_eq!(
            decode_container_view_result(bad),
            Err(ItemViewWireError::Malformed)
        );
    }
    assert_eq!(
        decode_container_view_result(&[0x08, 0x81, 0x80, 0x00, 0x00]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn command_9_takes_the_container_destination_only_under_capability_14() {
    let tree = ItemMoveSelection {
        equip_drop: true,
        container_tree: true,
    };
    let worst = ItemMoveIntent {
        source: handle(u64::MAX),
        destination: ItemMoveDestination::Container(handle(u64::MAX - 1)),
    };
    let bytes = encode_item_move_intent_for(&worst, tree).expect("container destination");
    // 11 for the source, 1 + 1 + 11 for the destination: within the capability 12 bound.
    assert_eq!(bytes.len(), 24);
    assert!(bytes.len() <= MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES);
    assert_eq!(decode_item_move_intent_for(&bytes, tree), Ok(worst));
    // Capability 14 alone in the selection (its closure guarantees 12) keeps the 29-byte bound.
    let only_tree = ItemMoveSelection {
        equip_drop: false,
        container_tree: true,
    };
    assert_eq!(decode_item_move_intent_for(&bytes, only_tree), Ok(worst));
    assert!(bytes.len() > MAX_ITEM_MOVE_INTENT_BYTES);

    // Without capability 14, field 5 fails closed both ways.
    for equip_drop in [false, true] {
        let selection = ItemMoveSelection {
            equip_drop,
            container_tree: false,
        };
        assert_eq!(
            encode_item_move_intent_for(&worst, selection),
            Err(ItemViewWireError::Malformed)
        );
    }
    assert_eq!(
        decode_item_move_intent_with_equip_drop(&bytes, true),
        Err(ItemViewWireError::Malformed)
    );
    // Under it, a zero or absent handle and a second destination fail closed.
    for bad in [
        &[0x08, 0x01, 0x2a, 0x00][..],
        &[0x08, 0x01, 0x2a, 0x02, 0x08, 0x00],
        &[0x08, 0x01, 0x2a, 0x02, 0x08, 0x02, 0x12, 0x00],
        &[0x08, 0x01, 0x2a, 0x02, 0x08, 0x02, 0x2a, 0x02, 0x08, 0x03],
    ] {
        assert_eq!(
            decode_item_move_intent_for(bad, tree),
            Err(ItemViewWireError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn live_handle_bound_adds_16_views_of_21_items() {
    assert_eq!(MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE, 637);
    assert_eq!(
        MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE,
        MAX_LIVE_ITEM_HANDLES + MAX_CONTAINER_VIEWS * (1 + MAX_CONTAINER_VIEW_ENTRIES)
    );
    // BAGS-0 §5: at least 382 (the capability 4 views and 16 views of 20 entries, besides the
    // containers themselves).
    const { assert!(MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE >= 382) };
}

#[test]
fn registries_bind_capability_14_and_its_limits() {
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
    let capability = find("capabilities", CAPABILITY_CONTAINER_TREE_V1);
    assert_eq!(capability["name"], "CONTAINER_TREE_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_CONTAINER_TREE_V1_REQUIRES)
    );
    assert_eq!(CAPABILITY_CONTAINER_TREE_V1_REQUIRES, [4, 12]);
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_CONTAINER_VIEW_INTENT])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_CONTAINER_VIEWS])
    );
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_CONTAINER_TREE_V1));

    let command = find("command_types", COMMAND_TYPE_CONTAINER_VIEW_INTENT);
    assert_eq!(command["name"], "CONTAINER_VIEW_INTENT");
    assert_eq!(command["capability"], CAPABILITY_CONTAINER_TREE_V1);
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_CONTAINER_VIEW_INTENT_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_CONTAINER_VIEW_RESULT_BYTES as u64)
    );
    // Command 9 keeps its owner and its bound: the container destination fits within it.
    let move_command = find("command_types", COMMAND_TYPE_ITEM_MOVE_INTENT);
    assert_eq!(move_command["capability"], 4);
    assert_eq!(
        move_command["max_payload_bytes"].as_u64(),
        Some(MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES as u64)
    );

    let domain = find("state_domains", STATE_DOMAIN_CONTAINER_VIEWS);
    assert_eq!(domain["name"], "CONTAINER_VIEWS");
    assert_eq!(domain["capability"], CAPABILITY_CONTAINER_TREE_V1);
    for (key, type_id) in [
        ("delta_types", DELTA_TYPE_CONTAINER_VIEWS_V1),
        ("snapshot_types", SNAPSHOT_TYPE_CONTAINER_VIEWS_V1),
    ] {
        let types = domain[key].as_array().expect(key);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0]["id"], type_id);
        assert_eq!(
            types[0]["max_payload_bytes"].as_u64(),
            Some(MAX_CONTAINER_VIEWS_BYTES as u64)
        );
        assert_eq!(
            types[0]["payload_schema"],
            "docs/contracts/protocol-oteryn/v1/container_tree_v1.proto#ContainerViewsV1"
        );
    }
    // Domains 9 and 11 keep capability 4.
    for id in [9, 11] {
        assert_eq!(find("state_domains", id)["capability"], 4);
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
    assert_eq!(limit("BAGS0-RL-03"), Some(MAX_CONTAINER_VIEWS as u64));
    assert_eq!(
        limit("BAGS0-RL-03-BYTES"),
        Some(MAX_CONTAINER_VIEWS_BYTES as u64)
    );
    assert_eq!(
        limit("GAMEITEM01-CONTAINER-ENTRIES-MAX"),
        Some(MAX_CONTAINER_VIEW_ENTRIES as u64)
    );
    assert_eq!(
        limit("ITEMV0-RL-03-CONTAINER-TREE"),
        Some(MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE as u64)
    );
    // Sessions without capability 14 keep ITEMV0-RL-03.
    assert_eq!(limit("ITEMV0-RL-03"), Some(MAX_LIVE_ITEM_HANDLES as u64));
}
