#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::content::native_gameplay::tests::activated_with_item_keys;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{BackpackEntry, CharacterBackpack, InventoryItem};
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CarrierError, ExactActorRef,
    MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;

const PRODUCTION_KEYS: &[u8] =
    include_bytes!("../../../../tools/content-schema/native-gameplay/item-keys.json");

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
    assert!(offered_ids(offered_capabilities(with.active())).contains(&CAPABILITY_ITEM_VIEW_MOVE_V1));
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
        runtime.bound_corpse(corpse.identity).unwrap().item.item_instance_id,
        CORPSE_ITEM
    );
    assert!(visible_corpses(&runtime, None).is_empty());

    let (mut unmapped, creature) = runtime_with_corpse(generation, true);
    unmapped
        .bind_corpse_item(creature, CORPSE_ITEM, "Item", "oteryn:item.unknown", "definition-r1")
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
