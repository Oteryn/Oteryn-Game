#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::combat_pickup::PickupContentError;
use crate::content::native_gameplay::test_support::activated_with_item_keys;
use crate::domain::currency::{COIN_STACK_MAXIMUM, Coin};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    BackpackEntry, CharacterBackpack, InventoryItem, ItemDefinitionFacts,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use {
    crate::foundation::{
        ChannelId, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, NodeId,
    },
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};

const PRODUCTION_KEYS: &[u8] =
    include_bytes!("../../../../../tools/content-schema/native-gameplay/item-keys.json");

fn offered_ids(offered: &[OfferedCapability]) -> Vec<u32> {
    offered.iter().map(|capability| capability.id).collect()
}

/// Part B sets the gate: production offers capability 4 exactly when the active generation pins
/// a non-empty Item key set.
#[test]
fn production_offers_capability_4_with_a_non_empty_item_key_set() {
    assert_eq!(offered_capabilities(None), PRODUCTION_OFFERED_CAPABILITIES);
    let with = activated_with_item_keys(Some(PRODUCTION_KEYS));
    assert_eq!(
        offered_capabilities(with.active()),
        ITEM_VIEW_OFFERED_CAPABILITIES
    );
    assert!(
        offered_ids(offered_capabilities(with.active())).contains(&CAPABILITY_ITEM_VIEW_MOVE_V1)
    );
    let without = activated_with_item_keys(None);
    assert_eq!(
        offered_capabilities(without.active()),
        PRODUCTION_OFFERED_CAPABILITIES
    );
    // Without the gate nothing offers it.
    assert_eq!(
        offered_capabilities_when(false, with.active()),
        PRODUCTION_OFFERED_CAPABILITIES
    );
}

/// With the gate set, capability 4 is offered exactly when the activated generation pins a
/// non-empty Item key set; a generation without one, an empty one or no generation offers
/// everything else.
#[test]
fn capability_4_is_offered_only_by_a_generation_with_a_non_empty_item_key_set() {
    assert_eq!(
        offered_capabilities_when(true, None),
        PRODUCTION_OFFERED_CAPABILITIES
    );
    let with = activated_with_item_keys(Some(PRODUCTION_KEYS));
    assert_eq!(
        offered_capabilities_when(true, with.active()),
        ITEM_VIEW_OFFERED_CAPABILITIES
    );
    assert_eq!(
        SelectedCapabilities::select(offered_capabilities_when(true, with.active()), &[4, 6])
            .unwrap()
            .as_slice(),
        [CAPABILITY_ITEM_VIEW_MOVE_V1, 6]
    );
    for keys in [
        None,
        Some(&br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[]}"#[..]),
    ] {
        let without = activated_with_item_keys(keys);
        let offered = offered_capabilities_when(true, without.active());
        assert!(!offered_ids(offered).contains(&CAPABILITY_ITEM_VIEW_MOVE_V1));
        assert_eq!(
            SelectedCapabilities::select(offered, &[4, 6])
                .unwrap()
                .as_slice(),
            [6]
        );
    }
}

/// Domain 9 names each durable backpack definition by the activated generation's index; a
/// definition the index does not name, at another revision or of another family, fails closed.
#[test]
fn domain_9_reads_definitions_through_the_activated_item_index() {
    let controller = activated_with_item_keys(Some(PRODUCTION_KEYS));
    let index = item_index(controller.active()).unwrap();
    let definition = |family: &str, key: &str, revision: &str| TypedDefinitionRef {
        family: family.to_owned(),
        production_key: key.to_owned(),
        revision_ref: revision.to_owned(),
    };
    let stored = |id: u8, definition: TypedDefinitionRef| InventoryItem {
        item_instance_id: [id; 16],
        definition,
        quantity: 1,
    };
    let backpack = CharacterBackpack {
        backpack: stored(
            1,
            definition("Item", "oteryn:item.tibia.i100", "definition-r1"),
        ),
        entries: vec![BackpackEntry {
            item: stored(
                2,
                definition("Item", "oteryn:item.tibia.i1000", "definition-r1"),
            ),
            placement_ordinal: 0,
        }],
        fluids: Default::default(),
    };
    let read = |backpack: &CharacterBackpack| {
        inventory_from_backpack(Some(backpack), |d| {
            index.definition_ref(&d.family, &d.production_key, &d.revision_ref)
        })
    };
    let items = read(&backpack).unwrap();
    let main = items.main_backpack.unwrap();
    assert_eq!(main.item_definition_ref.get(), 1);
    assert_eq!(
        items.entries[0].item_definition_ref,
        index
            .definition_ref("Item", "oteryn:item.tibia.i1000", "definition-r1")
            .unwrap()
    );
    for refused in [
        definition("Item", "oteryn:item.tibia.i100", "definition-r2"),
        definition("ItemType", "oteryn:item.tibia.i100", "definition-r1"),
        definition("Item", "oteryn:item.unknown", "definition-r1"),
    ] {
        let mut changed = backpack.clone();
        changed.entries[0].item.definition = refused;
        assert_eq!(read(&changed), None);
    }
    // A generation without an Item key set has no index, so nothing is read.
    assert!(item_index(activated_with_item_keys(None).active()).is_none());
}

const CORPSE_ITEM: [u8; 16] = [
    0x01, 0x90, 0, 0, 0, 0x41, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x41,
];
const CORPSE_KEY: &str = "oteryn:item.tibia.i1000";

fn uuid_v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ]
}

/// A Channel runtime pinned to `generation` (or to another digest when `pinned` is false), with
/// one projected creature corpse at (101, 100, 7).
fn runtime_with_corpse(
    generation: &crate::content::ActiveGeneration,
    pinned: bool,
) -> (ChannelRuntimeV1, ExactActorRef) {
    let identity = generation.identity();
    let world = identity.world_id();
    let server = if pinned {
        identity.server_artifact_digest()
    } else {
        [0xee; 32]
    };
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        server,
        identity.client_artifact_digest(),
        [3; 32],
        [4; 32],
        (100, 100, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(0x61)).unwrap(),
        NodeId::decode(&uuid_v7(0x62)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .unwrap();
    let creature = runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7,
        })
        .unwrap();
    runtime
        .commit_monster_lab_damage(creature, b"item-ref-corpse", 20)
        .unwrap();
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(creature).unwrap();
    death.project_committed_lethal(receipt).unwrap();
    (runtime, creature)
}

/// Domain 1 shows a projected corpse only once its corpse Item is bound, as a D85 object of
/// entity generation 0 whose `item_definition_ref` is the pinned index's; an unmapped definition,
/// another pinned generation or no generation shows none.
#[test]
fn a_bound_corpse_is_a_domain_1_object_named_by_the_pinned_index() {
    let controller = activated_with_item_keys(Some(PRODUCTION_KEYS));
    let generation = controller.active().unwrap();
    let index = item_index(Some(generation)).unwrap();
    let (mut runtime, creature) = runtime_with_corpse(generation, true);
    assert!(visible_corpses(&runtime, Some(generation)).is_empty());
    runtime
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1")
        .unwrap();
    let corpses = visible_corpses(&runtime, Some(generation));
    assert_eq!(corpses.len(), 1);
    let corpse = corpses[0];
    assert_eq!(
        corpse.kind,
        VisibleKind::Corpse {
            native_appearance_id: None,
            item_definition_ref: index
                .definition_ref("Item", CORPSE_KEY, "definition-r1")
                .unwrap()
        }
    );
    assert_eq!(corpse.identity, creature.corpse_identity());
    assert_ne!(corpse.identity, creature.placement_identity());
    assert_eq!(corpse.generation, 0);
    assert_eq!(
        (corpse.position.x, corpse.position.y, corpse.position.floor),
        (101, 100, 7)
    );
    assert_eq!(
        runtime
            .bound_corpse(corpse.identity)
            .unwrap()
            .item
            .item_instance_id,
        CORPSE_ITEM
    );
    assert!(visible_corpses(&runtime, None).is_empty());

    let (mut unmapped, creature) = runtime_with_corpse(generation, true);
    unmapped
        .bind_corpse_item(
            creature,
            CORPSE_ITEM,
            "Item",
            "oteryn:item.unknown",
            "definition-r1",
        )
        .unwrap();
    assert!(visible_corpses(&unmapped, Some(generation)).is_empty());

    let (mut other, creature) = runtime_with_corpse(generation, false);
    other
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1")
        .unwrap();
    assert!(visible_corpses(&other, Some(generation)).is_empty());
}

/// The binding is the KILL-REWARD part B interface: only a retained projection binds, rebinding
/// the same Item is a no-op, and another Item or an Item already bound elsewhere conflicts.
#[test]
fn a_corpse_binds_once_to_one_item() {
    let controller = activated_with_item_keys(Some(PRODUCTION_KEYS));
    let generation = controller.active().unwrap();
    let (mut runtime, creature) = runtime_with_corpse(generation, true);
    let live = runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 102,
            y: 100,
            floor: 7,
        })
        .unwrap();
    assert_eq!(
        runtime.bind_corpse_item(live, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1"),
        Err(CarrierError::CommittedLethalUnavailable)
    );
    runtime
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1")
        .unwrap();
    runtime
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1")
        .unwrap();
    assert_eq!(
        runtime.bind_corpse_item(creature, [7; 16], "Item", CORPSE_KEY, "definition-r1"),
        Err(CarrierError::CorpseProjectionConflict)
    );
    runtime
        .commit_monster_lab_damage(live, b"item-ref-corpse-2", 20)
        .unwrap();
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(live).unwrap();
    death.project_committed_lethal(receipt).unwrap();
    assert_eq!(
        runtime.bind_corpse_item(live, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1"),
        Err(CarrierError::CorpseProjectionConflict)
    );
    assert_eq!(runtime.visible_corpses().len(), 1);
    assert!(runtime.bound_corpse(live.corpse_identity()).is_none());
}

/// A take names a bound corpse entity and an Item instance entry; an unbound corpse, a dead
/// actor or another key shape is refused before any durable call and never yields an Item id.
#[test]
fn an_unbound_corpse_refuses_the_take_before_any_durable_call() {
    let controller = activated_with_item_keys(Some(PRODUCTION_KEYS));
    let generation = controller.active().unwrap();
    let (mut runtime, creature) = runtime_with_corpse(generation, true);
    let corpse = ItemKey::Entity(creature.corpse_identity());
    let entry = ItemKey::Instance([9; 16]);
    let rejected = |outcome: Result<([u8; 16], [u8; 16]), GroundPickupError>| {
        matches!(
            outcome,
            Err(GroundPickupError::Transfer(
                ItemTransferError::AuthorityRejected
            ))
        )
    };
    assert!(rejected(corpse_for_take(&runtime, false, corpse, entry)));
    runtime
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", CORPSE_KEY, "definition-r1")
        .unwrap();
    assert_eq!(
        corpse_for_take(&runtime, false, corpse, entry).unwrap(),
        (CORPSE_ITEM, [9; 16])
    );
    assert!(rejected(corpse_for_take(&runtime, true, corpse, entry)));
    assert!(rejected(corpse_for_take(&runtime, false, entry, entry)));
    assert!(rejected(corpse_for_take(&runtime, false, corpse, corpse)));
    assert!(rejected(corpse_for_take(
        &runtime,
        false,
        ItemKey::Entity(creature.placement_identity()),
        entry
    )));
}

/// Codex 4202915395: a corpse entry's facts are the pinned generation's canonical Item profile,
/// so real creature loot (the rat's cheese) and the real backpack resolve, while a definition
/// the generation does not profile, or names only by its authoring key, fails closed.
#[test]
fn corpse_take_facts_are_the_generation_item_profiles() {
    use crate::content::native_gameplay::NativeGameplayInput;
    use crate::content::{
        ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
        activate_native_entry_room_with_gameplay, qualify_native_entry_room_with_gameplay,
    };
    // The committed production manifest's pins (its 121 canonical Item profiles among them), on
    // the accepted entry room rather than the separately produced source world.
    let manifest = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server"
    ))
    .join("../../content/spells.manifest.json");
    let mut supplied = NativeGameplayInput::from_manifest(&manifest).unwrap();
    supplied.source_world = None;
    let world = activated_with_item_keys(None)
        .active()
        .unwrap()
        .identity()
        .world_id();
    let room = qualify_native_entry_room_with_gameplay(world, &supplied).unwrap();
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &supplied,
    )
    .unwrap();
    let native = controller.active().unwrap().native_gameplay().unwrap();
    let item = |key: &str| TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: key.to_owned(),
        revision_ref: "definition-r1".to_owned(),
    };
    let cheese = item("oteryn:item.tibia.i3607");
    assert_eq!(
        generation_item_facts(native, &cheese).unwrap(),
        ItemDefinitionFacts {
            definition: cheese.clone(),
            weight: Some(400),
            stack: ItemStackClass::Stackable {
                proven_maximum: Some(100)
            },
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    );
    let backpack = item("oteryn:item.tibia.i2854");
    assert_eq!(
        generation_item_facts(native, &backpack).unwrap(),
        ItemDefinitionFacts {
            definition: backpack.clone(),
            weight: Some(1_800),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(20),
            container_slot_equip_pattern: true,
        }
    );
    assert_eq!(
        NativeItemWeights(native).generation_digest(),
        native.source_digest()
    );
    assert_eq!(NativeItemWeights(native).weight_of(&cheese), Ok(400));
    assert_eq!(NativeItemWeights(native).weight_of(&backpack), Ok(1_800));
    assert_eq!(
        NativeItemWeights(native).weight_of(&TypedDefinitionRef {
            revision_ref: "another-generation-revision".to_owned(),
            ..cheese.clone()
        }),
        Err(ItemWeightRefused::DefinitionMismatch)
    );
    let not_found = |definition: TypedDefinitionRef| {
        generation_item_facts(native, &definition) == Err(PickupContentError::DefinitionNotFound)
    };
    // Every coin of the closed D176 table resolves to its stack maximum and its Global client
    // weight (gold, platinum and crystal 0.10 oz, in hundredths of an ounce).
    for coin in Coin::ALL {
        let definition = item(coin.production_key());
        assert_eq!(
            generation_item_facts(native, &definition).unwrap(),
            ItemDefinitionFacts {
                definition: definition.clone(),
                weight: Some(10),
                stack: ItemStackClass::Stackable {
                    proven_maximum: Some(COIN_STACK_MAXIMUM)
                },
                container_capacity: None,
                container_slot_equip_pattern: false,
            },
            "{coin:?}"
        );
    }
    assert!(not_found(TypedDefinitionRef {
        revision_ref: "definition-r2".to_owned(),
        ..cheese.clone()
    }));
    assert!(not_found(TypedDefinitionRef {
        family: "Creature".to_owned(),
        ..cheese.clone()
    }));
    assert!(not_found(TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: "candidate:item/3607".to_owned(),
        revision_ref: "spell-p2-r21".to_owned(),
    }));
}

const LOOT_TABLES: &[u8] =
    include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json");

/// The `(key, revision, min_count, max_count)` of every entry of the pinned loot table `table`.
fn loot_entries(table: &serde_json::Value) -> Vec<(String, String, u64, u64)> {
    table["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["item"]["key"].as_str().unwrap().to_owned(),
                entry["item"]["revision"].as_str().unwrap().to_owned(),
                entry["min_count"].as_u64().unwrap(),
                entry["max_count"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn pinned_loot_tables() -> Vec<serde_json::Value> {
    let section: serde_json::Value = serde_json::from_slice(LOOT_TABLES).unwrap();
    section["tables"].as_array().unwrap().clone()
}

/// A Cyclops corpse holding one stack of every entry of its real pinned loot table, each at its
/// maximum count (Crystal 201189f8 `data-otservbr-global/monster/giants/cyclops.lua`).
fn cyclops_corpse() -> Vec<BackpackEntry> {
    let table = pinned_loot_tables()
        .into_iter()
        .find(|table| table["identity"]["key"] == "oteryn:loot.creature.cyclops")
        .unwrap();
    loot_entries(&table)
        .into_iter()
        .enumerate()
        .map(|(ordinal, (key, revision, _, max_count))| {
            corpse_entry(ordinal, &key, &revision, u32::try_from(max_count).unwrap())
        })
        .collect()
}

fn corpse_entry(ordinal: usize, key: &str, revision: &str, quantity: u32) -> BackpackEntry {
    let mut item_instance_id = [0x5c; 16];
    item_instance_id[15] = u8::try_from(ordinal).unwrap();
    BackpackEntry {
        item: InventoryItem {
            item_instance_id,
            definition: TypedDefinitionRef {
                family: "Item".to_owned(),
                production_key: key.to_owned(),
                revision_ref: revision.to_owned(),
            },
            quantity,
        },
        placement_ordinal: u64::try_from(ordinal).unwrap() + 1,
    }
}

/// CORPSELOOT: every entry of a real Cyclops corpse is shown, in loot order, with its count.
#[test]
fn cyclops_corpse_shows_every_real_loot_entry() {
    let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    let corpse = cyclops_corpse();
    assert_eq!(corpse.len(), 13);
    let shown = corpse_view_items(&index, &corpse);
    assert_eq!(
        shown
            .iter()
            .map(|item| (item.key, item.count.get()))
            .collect::<Vec<_>>(),
        corpse
            .iter()
            .map(|entry| (
                ItemKey::Instance(entry.item.item_instance_id),
                entry.item.quantity
            ))
            .collect::<Vec<_>>()
    );
}

/// CORPSELOOT (E2E nbgh71): one entry the pinned index does not name, or one with no quantity,
/// never hides the rest of the corpse. Per Crystal the corpse opens and shows the items it has;
/// each refused entry is reported with its item id and the reason.
#[test]
fn one_unnamed_or_empty_corpse_entry_never_hides_the_rest() {
    let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    let mut corpse = cyclops_corpse();
    let real = corpse.clone();
    corpse.insert(
        2,
        corpse_entry(40, "oteryn:item.tibia.i99999999", "definition-r1", 1),
    );
    corpse.insert(5, corpse_entry(41, "oteryn:item.tibia.i3031", "definition-r1", 0));
    let mut shown = Vec::new();
    let lines = oteryn_error_codes::capture(|| shown = corpse_view_items(&index, &corpse));
    assert_eq!(
        shown.iter().map(|item| item.key).collect::<Vec<_>>(),
        real.iter()
            .map(|entry| ItemKey::Instance(entry.item.item_instance_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].contains("event=corpse_entry_refused"), "{lines:?}");
    assert!(lines[0].contains("item=oteryn:item.tibia.i99999999"), "{lines:?}");
    assert!(lines[0].contains("reason=not_in_item_index"), "{lines:?}");
    assert!(lines[1].contains("item=oteryn:item.tibia.i3031"), "{lines:?}");
    assert!(lines[1].contains("reason=zero_quantity"), "{lines:?}");
}

/// CORPSELOOT: the whole defect class over the pinned content. Every entry of every species'
/// loot table is an Item the pinned index names at exactly its revision, with a non-empty count
/// range, so no real drop is refused by the corpse view. A failure lists every species.
#[test]
fn every_species_loot_entry_is_named_by_the_pinned_item_index() {
    let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    let tables = pinned_loot_tables();
    assert_eq!(tables.len(), 1292);
    let failing: Vec<String> = tables
        .iter()
        .filter_map(|table| {
            let bad: Vec<String> = loot_entries(table)
                .into_iter()
                .filter(|(key, revision, min_count, max_count)| {
                    index.definition_ref("Item", key, revision).is_none()
                        || *min_count == 0
                        || max_count < min_count
                })
                .map(|(key, _, min_count, max_count)| format!("{key}[{min_count}..={max_count}]"))
                .collect();
            (!bad.is_empty())
                .then(|| format!("{}: {}", table["identity"]["key"], bad.join(", ")))
        })
        .collect();
    assert!(failing.is_empty(), "species with unmappable loot: {failing:#?}");
}
