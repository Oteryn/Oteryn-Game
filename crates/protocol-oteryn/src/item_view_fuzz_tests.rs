#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_item_target() {
    crate::decoder_fuzz_support::codec(
        "decode_item_target",
        |rng| handle(rng.u64().max(1)),
        decode_item_target,
        |v| encode_item_target(*v),
    );
}

#[test]
fn fuzz_decode_character_inventory() {
    crate::decoder_fuzz_support::codec(
        "decode_character_inventory",
        |rng| inventory(rng.below(8) as usize, small),
        decode_character_inventory,
        |v| encode_character_inventory(v).expect("inventory"),
    );
}

#[test]
fn fuzz_decode_open_container() {
    crate::decoder_fuzz_support::codec(
        "decode_open_container",
        |rng| container(rng.below(17) as usize, small),
        decode_open_container,
        |v| encode_open_container(v).expect("container"),
    );
}

#[test]
fn fuzz_decode_item_move_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_item_move_intent",
        |rng| ItemMoveIntent {
            source: handle(rng.u64().max(1)),
            destination: ItemMoveDestination::MainBackpack,
        },
        decode_item_move_intent,
        |v| encode_item_move_intent(v).expect("move"),
    );
}

#[test]
fn fuzz_decode_item_move_result() {
    crate::decoder_fuzz_support::codec(
        "decode_item_move_result",
        |rng| *rng.pick(&[ItemMoveOutcome::Moved, ItemMoveOutcome::Rejected]),
        decode_item_move_result,
        |v| encode_item_move_result(*v).expect("result"),
    );
}

#[test]
fn fuzz_selected_item_view_decoders() {
    for equip_drop in [false, true] {
        crate::decoder_fuzz_support::codec(
            "decode_character_inventory_with_equip_drop",
            |rng| {
                let mut value = inventory(rng.below(8) as usize, small);
                if equip_drop {
                    value.equipment.push(EquippedItem {
                        slot: EquipmentSlot::Head,
                        item: small(900),
                    });
                }
                value
            },
            |b| decode_character_inventory_with_equip_drop(b, equip_drop),
            |v| encode_character_inventory_with_equip_drop(v, equip_drop).expect("inventory"),
        );
        crate::decoder_fuzz_support::codec(
            "decode_item_move_intent_with_equip_drop",
            |rng| ItemMoveIntent {
                source: handle(rng.u64().max(1)),
                destination: if equip_drop {
                    ItemMoveDestination::Equipment(EquipmentSlot::Head)
                } else {
                    ItemMoveDestination::MainBackpack
                },
            },
            |b| decode_item_move_intent_with_equip_drop(b, equip_drop),
            |v| encode_item_move_intent_with_equip_drop(v, equip_drop).expect("move"),
        );
        crate::decoder_fuzz_support::codec(
            "decode_item_move_result_with_equip_drop",
            |rng| {
                if equip_drop && rng.bool() {
                    ItemMoveOutcome::SlotMismatch
                } else {
                    ItemMoveOutcome::Moved
                }
            },
            |b| decode_item_move_result_with_equip_drop(b, equip_drop),
            |v| encode_item_move_result_with_equip_drop(*v, equip_drop).expect("result"),
        );
        for container_tree in [false, true] {
            for world_interaction in [false, true] {
                let selection = ItemMoveSelection {
                    equip_drop,
                    container_tree,
                    world_interaction,
                };
                crate::decoder_fuzz_support::codec(
                    "decode_item_move_intent_for",
                    |rng| ItemMoveIntent {
                        source: handle(rng.u64().max(1)),
                        destination: if container_tree {
                            ItemMoveDestination::Container(handle(900))
                        } else if equip_drop || world_interaction {
                            ItemMoveDestination::Ground(WorldTilePosition {
                                x: rng.u64() as i32,
                                y: rng.u64() as i32,
                                floor: rng.u64() as i16,
                            })
                        } else {
                            ItemMoveDestination::MainBackpack
                        },
                    },
                    |b| decode_item_move_intent_for(b, selection),
                    |v| encode_item_move_intent_for(v, selection).expect("move"),
                );
                crate::decoder_fuzz_support::codec(
                    "decode_item_move_result_for",
                    |rng| {
                        if world_interaction && rng.bool() {
                            ItemMoveOutcome::Blocked
                        } else {
                            ItemMoveOutcome::Moved
                        }
                    },
                    |b| decode_item_move_result_for(b, selection),
                    |v| encode_item_move_result_for(*v, selection).expect("result"),
                );
            }
        }
    }
}
