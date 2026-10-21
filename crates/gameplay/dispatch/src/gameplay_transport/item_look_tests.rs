#![allow(clippy::unwrap_used)]

use super::super::item_view::{InventoryItems, ItemKey};
use super::*;
use crate::content::native_gameplay::test_support::{
    activated_with_item_keys, activated_with_loot_tables,
};
use std::num::NonZeroU32;
use {
    crate::foundation::{ChannelId, NodeId, WorldId},
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
};

fn id(tag: u8) -> [u8; 16] {
    [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

fn runtime(generation: &ActiveGeneration, pinned: bool) -> ChannelRuntimeV1 {
    let identity = generation.identity();
    runtime_with_binding(
        generation,
        pinned,
        identity.world_id(),
        generation.activation_sequence(),
    )
}

fn runtime_with_binding(
    generation: &ActiveGeneration,
    pinned: bool,
    world: WorldId,
    sequence: u64,
) -> ChannelRuntimeV1 {
    let identity = generation.identity();
    let pin = ChannelContentPin::from_activation(
        world,
        sequence,
        if pinned {
            identity.server_artifact_digest()
        } else {
            [0xee; 32]
        },
        identity.client_artifact_digest(),
        [3; 32],
        [4; 32],
        (100, 100, 7),
    );
    ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(1)).unwrap(),
        NodeId::decode(&id(2)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .unwrap()
}

#[test]
fn observed_target_requires_live_handle_exact_binding_and_current_pin() {
    let world = activated_with_item_keys(None)
        .active()
        .unwrap()
        .identity()
        .world_id();
    let (controller, _) = activated_with_loot_tables(world, None);
    let generation = controller.active().unwrap();
    let pinned = runtime(generation, true);
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3607".into(),
        revision_ref: "definition-r1".into(),
    };
    let index = pinned_index(&pinned, Some(generation)).unwrap();
    let item = ViewItem {
        key: ItemKey::Instance(id(3)),
        item_definition_ref: index
            .definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
            .unwrap(),
        count: NonZeroU32::new(2).unwrap(),
        sub_type: 0,
    };
    let mut views = SessionItemView::resume(Default::default());
    views
        .snapshot(
            InventoryItems {
                main_backpack: Some(ViewItem {
                    key: ItemKey::Instance(id(5)),
                    item_definition_ref: index
                        .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
                        .unwrap(),
                    count: NonZeroU32::MIN,
                    sub_type: 0,
                }),
                entries: vec![item],
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let target = views.table().handle(&item.key).unwrap();
    let observe = || {
        Some(ObservedLookItem {
            item,
            definition: &definition,
        })
    };
    assert!(matches!(
        examine_observed_item(&pinned, Some(generation), &views, target, observe()),
        ItemLookOutcome::Text(_)
    ));
    assert_eq!(
        examine_observed_item(&pinned, Some(generation), &views, target, None),
        ItemLookOutcome::Unavailable
    );
    assert_eq!(
        examine_observed_item(
            &runtime(generation, false),
            Some(generation),
            &views,
            target,
            observe()
        ),
        ItemLookOutcome::Unavailable
    );
    assert_eq!(
        examine_observed_item(&pinned, None, &views, target, observe()),
        ItemLookOutcome::Unavailable
    );
    for foreign in [
        runtime_with_binding(
            generation,
            true,
            WorldId::decode(&id(9)).unwrap(),
            generation.activation_sequence(),
        ),
        runtime_with_binding(
            generation,
            true,
            world,
            generation.activation_sequence() + 1,
        ),
    ] {
        assert_eq!(
            examine_observed_item(&foreign, Some(generation), &views, target, observe()),
            ItemLookOutcome::Unavailable
        );
    }
    for changed in [
        ViewItem {
            key: ItemKey::Instance(id(4)),
            ..item
        },
        ViewItem {
            item_definition_ref: NonZeroU32::MAX,
            ..item
        },
    ] {
        assert_eq!(
            examine_observed_item(
                &pinned,
                Some(generation),
                &views,
                target,
                Some(ObservedLookItem {
                    item: changed,
                    definition: &definition
                })
            ),
            ItemLookOutcome::Unavailable
        );
    }
    for definition in [
        TypedDefinitionRef {
            family: "Creature".into(),
            ..definition.clone()
        },
        TypedDefinitionRef {
            production_key: "oteryn:item.not-admitted".into(),
            ..definition.clone()
        },
        TypedDefinitionRef {
            revision_ref: "definition-r2".into(),
            ..definition.clone()
        },
    ] {
        assert_eq!(
            examine_observed_item(
                &pinned,
                Some(generation),
                &views,
                target,
                Some(ObservedLookItem {
                    item,
                    definition: &definition
                })
            ),
            ItemLookOutcome::Unavailable
        );
    }
    // An indexed definition without an admitted presentation profile has no fallback text.
    let keys: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../tools/content-schema/native-gameplay/item-keys.json"
    ))
    .unwrap();
    let native = generation.native_gameplay().unwrap();
    let unprofiled = keys["records"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|row| {
            let key = row[0].as_str().unwrap();
            let revision = row[1].as_str().unwrap();
            native
                .item_policy(key, revision)
                .is_none()
                .then(|| TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                })
        })
        .unwrap();
    let unprofiled_item = ViewItem {
        item_definition_ref: index
            .definition_ref(
                &unprofiled.family,
                &unprofiled.production_key,
                &unprofiled.revision_ref,
            )
            .unwrap(),
        ..item
    };
    assert_eq!(
        examine_observed_item(
            &pinned,
            Some(generation),
            &views,
            target,
            Some(ObservedLookItem {
                item: unprofiled_item,
                definition: &unprofiled,
            })
        ),
        ItemLookOutcome::Unavailable
    );
    // Retirement changes only local owner liveness: every target/content fact stays valid.
    let mut retiring = runtime(generation, true);
    assert!(retiring.owner_fence().is_ok());
    assert!(matches!(
        examine_observed_item(&retiring, Some(generation), &views, target, observe()),
        ItemLookOutcome::Text(_)
    ));
    retiring.retire_owner_cycle();
    assert!(retiring.owner_fence().is_err());
    assert_eq!(retiring.content_pin(), pinned.content_pin());
    assert_eq!(
        examine_observed_item(&retiring, Some(generation), &views, target, observe()),
        ItemLookOutcome::Unavailable
    );
    views.snapshot(InventoryItems::default(), None).unwrap();
    assert_eq!(
        examine_observed_item(&pinned, Some(generation), &views, target, observe()),
        ItemLookOutcome::Unavailable
    );
}
