#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::content::native_gameplay::tests::activated_with_item_keys;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{BackpackEntry, CharacterBackpack, InventoryItem};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;

const PRODUCTION_KEYS: &[u8] =
    include_bytes!("../../../../tools/content-schema/native-gameplay/item-keys.json");

fn offered_ids(offered: &[OfferedCapability]) -> Vec<u32> {
    offered.iter().map(|capability| capability.id).collect()
}

/// Until Part B sets the gate, production does not offer capability 4, even for a generation
/// with a non-empty Item key set.
#[test]
fn production_does_not_offer_capability_4_yet() {
    let with = activated_with_item_keys(Some(PRODUCTION_KEYS));
    for generation in [None, with.active()] {
        let offered = offered_capabilities(generation);
        assert_eq!(offered, PRODUCTION_OFFERED_CAPABILITIES);
        assert!(!offered_ids(offered).contains(&CAPABILITY_ITEM_VIEW_MOVE_V1));
    }
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
