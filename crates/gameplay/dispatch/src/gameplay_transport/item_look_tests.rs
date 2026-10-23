#![allow(clippy::unwrap_used, clippy::panic)]

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
fn player_revalidation_requires_the_same_visible_actor_and_session() {
    use crate::foundation::MovementLocalPosition;

    let controller = activated_with_item_keys(None);
    let mut runtime = runtime(controller.active().unwrap(), true);
    let observer_session = GameSessionId::decode(&id(3)).unwrap();
    let target_session = GameSessionId::decode(&id(4)).unwrap();
    let reservation = runtime.reserve_fresh_session(observer_session).unwrap();
    let observer = runtime.commit_fresh_session(reservation).unwrap();
    runtime.initialize_first_entry_position(observer).unwrap();
    let reservation = runtime.reserve_fresh_session(target_session).unwrap();
    let target_actor = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_first_entry_position(target_actor)
        .unwrap();
    let target = runtime
        .visible_entities()
        .players
        .into_iter()
        .find(|entry| entry.actor == target_actor)
        .unwrap();
    let target = EntityRef {
        identity: target.actor.placement_identity(),
        generation: target.generation,
    };
    let Ok(RuntimeLook::Player { actor, session, .. }) =
        runtime_look_creature(&mut runtime, None, observer, target)
    else {
        panic!("the visible target resolves before the durable read");
    };
    assert_eq!((actor, session), (target_actor, target_session));
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, actor, session),
        Ok(())
    );
    let unavailable = Err(ItemLookRefusal::NotAvailable);
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, observer, session),
        unavailable
    );
    assert_eq!(
        revalidate_look_player(
            &mut runtime,
            None,
            observer,
            target,
            actor,
            observer_session
        ),
        unavailable
    );

    // Movement after resolution invalidates the observation; coming back restores it.
    let original = runtime.read_actor_position(actor).unwrap().position();
    runtime
        .place_respawned_player(
            actor,
            session,
            MovementLocalPosition {
                x: original.x + 20,
                ..original
            },
        )
        .unwrap();
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, actor, session),
        unavailable
    );
    runtime
        .place_respawned_player(actor, session, original)
        .unwrap();
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, actor, session),
        Ok(())
    );
    runtime.remove_test_actor(actor).unwrap();
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, actor, session),
        unavailable
    );
    // Reusing the departed session's slot must not revive the old observation.
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let replacement = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_first_entry_position(replacement)
        .unwrap();
    assert_ne!(replacement, actor);
    assert_eq!(
        revalidate_look_player(&mut runtime, None, observer, target, actor, session),
        unavailable
    );
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

// LOOK-2 (CP D1143, ITEMLOOK1-RL-01): 10 looks per second per GameSession, a sliding window.
#[test]
fn the_look_window_admits_10_per_sliding_second() {
    let start = Instant::now();
    let mut window = LookWindow::EMPTY;
    for n in 0..10_u64 {
        assert!(
            window.admit(start + Duration::from_millis(n * 10)),
            "look {n}"
        );
    }
    // The 11th within the second, and again just before the first slot frees.
    assert!(!window.admit(start + Duration::from_millis(100)));
    assert!(!window.admit(start + Duration::from_millis(999)));
    // One second after the first look its slot is free again, and only that one.
    assert!(window.admit(start + Duration::from_millis(1_000)));
    assert!(!window.admit(start + Duration::from_millis(1_005)));
    // After the second look's second.
    assert!(window.admit(start + Duration::from_millis(1_010)));
    // A refused look spends nothing: a full second later the whole window is free.
    let later = start + Duration::from_secs(3);
    for n in 0..10_u64 {
        assert!(window.admit(later + Duration::from_millis(n)), "look {n}");
    }
    assert!(!window.admit(later + Duration::from_millis(10)));
    assert_eq!(ITEM_LOOK_COMMANDS_PER_SECOND, 10);
}

// LOOK-2: an actor is examined only within the reference view (VIS-1: 18 x 14, the observer
// at x offset 8 and y offset 6), never from a cached session viewport.
#[test]
fn an_actor_is_in_view_only_within_the_reference_window() {
    let me = ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let at = |x: i32, y: i32| ActorPosition { x, y, floor: 7 };
    for visible in [
        at(100, 100),
        at(92, 94),
        at(109, 107),
        at(92, 107),
        at(109, 94),
    ] {
        assert!(in_view(me, visible), "{visible:?}");
    }
    for hidden in [at(91, 100), at(110, 100), at(100, 93), at(100, 108)] {
        assert!(!in_view(me, hidden), "{hidden:?}");
    }
}

// LOOK-2: the actor texts. A refusal names nothing; a text is never empty and never over the
// 512-byte result bound.
#[test]
fn actor_texts_name_the_actor_and_stay_within_512_bytes() {
    use oteryn_protocol_oteryn::item_look::MAX_ITEM_LOOK_TEXT_BYTES;
    let text = |looked: Looked<'_>| match look_text(looked) {
        ItemLookResult::Text(text) => text,
        refused => panic!("refused {refused:?}"),
    };
    assert_eq!(text(Looked::Creature { name: "a rat" }), "You see a rat.");
    assert_eq!(
        text(Looked::Player {
            name: "Mole",
            level: 8,
            vocation: Some("knight"),
        }),
        "You see Mole (Level 8, knight)."
    );
    assert_eq!(
        text(Looked::Player {
            name: "Mole",
            level: 1,
            vocation: None,
        }),
        "You see Mole (Level 1)."
    );
    assert_eq!(
        text(Looked::Yourself {
            level: 8,
            vocation: Some("knight"),
        }),
        "You see yourself (Level 8, knight)."
    );
    assert_eq!(
        text(Looked::Yourself {
            level: 3,
            vocation: None,
        }),
        "You see yourself (Level 3)."
    );
    for blank in ["", " ", "\n"] {
        assert_eq!(
            look_text(Looked::Creature { name: blank }),
            ItemLookResult::Refused(ItemLookRefusal::NotAvailable)
        );
        assert_eq!(
            look_text(Looked::Player {
                name: blank,
                level: 1,
                vocation: None,
            }),
            ItemLookResult::Refused(ItemLookRefusal::NotAvailable)
        );
    }
    // An over-long name is cut on a character boundary at the bound.
    let long = "é".repeat(MAX_ITEM_LOOK_TEXT_BYTES);
    let cut = text(Looked::Creature { name: &long });
    assert!(cut.len() <= MAX_ITEM_LOOK_TEXT_BYTES);
    assert!(cut.len() >= MAX_ITEM_LOOK_TEXT_BYTES - 1);
    assert!(cut.starts_with("You see é"));
}

// LOOK-2: every vocation has its display name, lower case as the look text shows it.
#[test]
fn every_vocation_has_a_look_name() {
    use crate::spell::Vocation;
    let names = [
        (Vocation::Druid, "druid"),
        (Vocation::ElderDruid, "elder druid"),
        (Vocation::Sorcerer, "sorcerer"),
        (Vocation::MasterSorcerer, "master sorcerer"),
        (Vocation::Knight, "knight"),
        (Vocation::EliteKnight, "elite knight"),
        (Vocation::Paladin, "paladin"),
        (Vocation::RoyalPaladin, "royal paladin"),
        (Vocation::Monk, "monk"),
        (Vocation::ExaltedMonk, "exalted monk"),
    ];
    for (vocation, name) in names {
        assert_eq!(vocation_name(vocation), name);
    }
}

// LOOK-2: a map item's wire `item_definition_ref` names its typed definition back, so a map
// entry is examined from the same pinned index that numbered it.
#[test]
fn a_definition_ref_names_its_typed_definition_back() {
    let world = activated_with_item_keys(None)
        .active()
        .unwrap()
        .identity()
        .world_id();
    let (controller, _) = activated_with_loot_tables(world, None);
    let generation = controller.active().unwrap();
    let pinned = runtime(generation, true);
    let index = pinned_index(&pinned, Some(generation)).unwrap();
    let reference = index
        .definition_ref("Item", "oteryn:item.tibia.i3607", "definition-r1")
        .unwrap();
    assert_eq!(
        typed_definition(index, reference),
        Some(TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3607".into(),
            revision_ref: "definition-r1".into(),
        })
    );
    assert_eq!(typed_definition(index, NonZeroU32::MAX), None);
}

#[test]
fn item_look_window_matches_its_registered_resource_limit() {
    let registry: serde_json::Value =
        serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON).unwrap();
    let row = registry["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "ITEMLOOK1-RL-01")
        .unwrap();
    assert_eq!(
        row["hard_maximum"].as_u64(),
        Some(ITEM_LOOK_COMMANDS_PER_SECOND as u64)
    );
    assert_eq!(row["configurable_range"]["minimum"], row["hard_maximum"]);
    assert_eq!(row["configurable_range"]["maximum"], row["hard_maximum"]);
    assert_eq!(row["failure_category"], "CAPACITY_EXCEEDED");
    assert_eq!(row["client_visible"], true);
}
