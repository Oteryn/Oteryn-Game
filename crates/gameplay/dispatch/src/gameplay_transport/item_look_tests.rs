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
            look_distance: 1,
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
                    definition: &definition,
                    look_distance: 1,
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
                    definition: &definition,
                    look_distance: 1,
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
                look_distance: 1,
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

#[test]
fn item_look_uses_the_map_send_window_on_every_floor_and_the_canary_near_split() {
    use crate::map::overlay::TilePos;
    for floor in -15_i8..=0 {
        let origin = TilePos {
            x: 100,
            y: 100,
            floor,
        };
        let layout = view::WindowLayout::of(origin).unwrap();
        let actor = ActorPosition {
            x: 100,
            y: 100,
            floor: i16::from(floor),
        };
        for z in -15_i8..=0 {
            for x in 88..=112 {
                for y in 88..=112 {
                    let pos = TilePos { x, y, floor: z };
                    let target = ActorPosition {
                        x: i32::from(x),
                        y: i32::from(y),
                        floor: i16::from(z),
                    };
                    assert_eq!(
                        in_view(actor, target),
                        layout.slot_of(pos).is_some(),
                        "{actor:?} -> {target:?}"
                    );
                    assert_eq!(
                        look_distance(actor, target) <= 1,
                        floor == z && x.abs_diff(100) <= 1 && y.abs_diff(100) <= 1
                    );
                }
            }
        }
    }
    let actor = ActorPosition {
        x: i32::MIN,
        y: 0,
        floor: 7,
    };
    let target = ActorPosition {
        x: i32::MAX,
        y: 0,
        floor: 6,
    };
    assert_eq!(look_distance(actor, target), i32::MAX);
}

#[test]
fn item_look_details_stay_within_the_wire_bound_on_a_utf8_boundary() {
    let text = bounded_text(format!("You see {}", "é".repeat(MAX_ITEM_LOOK_TEXT_BYTES)));
    assert!(text.len() <= MAX_ITEM_LOOK_TEXT_BYTES);
    assert!(text.ends_with('é'));
    assert!(
        oteryn_protocol_oteryn::item_look::encode_item_look_result_for(
            &ItemLookResult::Text(text),
            oteryn_protocol_oteryn::item_look::ItemLookSelection { extended: true },
        )
        .is_ok()
    );
}

fn with_reading_map(
    text: &str,
    test: impl FnOnce(
        &ActiveGeneration,
        crate::map::boot::BundleWorld,
        oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ),
) {
    with_reading_map_entries(text, 0, test);
}

fn with_reading_map_entries(
    text: &str,
    extra_entries: usize,
    test: impl FnOnce(
        &ActiveGeneration,
        crate::map::boot::BundleWorld,
        oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ),
) {
    use crate::map::boot::{BootPins, boot, tests::bundle_with_item_ids};
    use crate::map::facts::ItemDefinition;
    use crate::map::overlay::TilePos;
    let world = WorldId::decode(&id(42)).unwrap();
    let (controller, _) = activated_with_loot_tables(world, None);
    let generation = controller.active().unwrap();
    let index = generation.native_gameplay().unwrap().item_index().unwrap();
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1").unwrap();
    let (bytes, mut pins) = bundle_with_item_ids(
        (
            "oteryn:item.tibia.i2854",
            reference("oteryn:item.tibia.i2854").get() - 1,
        ),
        (
            "oteryn:item.tibia.i2012",
            reference("oteryn:item.tibia.i2012").get() - 1,
        ),
    );
    let mut decoded = oteryn_world_bundle::bundle::read(&bytes).unwrap();
    // The non-distance-readable box is also text-bound, so it is a genuine
    // base-ordinal target despite the definition being pickupable.
    decoded.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 6)
        .unwrap()
        .items
        .last_mut()
        .unwrap()
        .attrs
        .text = Some(text.into());
    let sign = decoded.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 7)
        .unwrap();
    sign.items.last_mut().unwrap().attrs.text = Some(text.into());
    sign.items.extend(std::iter::repeat_n(
        sign.items.last().unwrap().clone(),
        extra_entries,
    ));
    let bytes = oteryn_world_bundle_compiler::bundle::write(
        &decoded.manifest,
        &decoded.sectors,
        &decoded.spawns,
    )
    .unwrap();
    pins.digest = oteryn_world_bundle::bundle::read(&bytes).unwrap().digest;
    let map_revision = format!(
        "sha256:{}",
        pins.digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let map = boot(
        &bytes,
        &BootPins {
            bundle: pins,
            map_revision,
            start: TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
        },
        world,
        ChannelId::decode(&id(1)).unwrap(),
        |key| {
            let record = generation.native_gameplay()?.item_policy(key, "definition-r1")?.record();
            Some(ItemDefinition {
                reference: reference(key),
                solid: record.attributes.blocks_movement,
                blocks_projectile: record.attributes.blocks_projectile.unwrap_or(true),
                pickupable: matches!(&record.semantics.physical, crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true)),
                door: false,
            })
        },
    )
    .unwrap();
    let key = view::placement_key(
        TilePos {
            x: 7,
            y: 0,
            floor: -7,
        },
        1,
    )
    .unwrap();
    let mut placement = map.overlay().base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    test(
        generation,
        map,
        oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: 0,
        },
    );
}

fn read_map_sign(
    generation: &ActiveGeneration,
    map: &crate::map::boot::BundleWorld,
    target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    actor: ActorPosition,
) -> Result<ItemLookResult, ItemLookRefusal> {
    let facts = map.facts().pinned();
    let source = super::super::world_map::MapViewSource {
        overlay: map.overlay(),
        facts: &facts,
        content_generation: generation.identity().client_artifact_digest(),
        reset_epoch: 0,
    };
    examine_world_object(
        &source,
        actor,
        target,
        generation,
        generation.native_gameplay().unwrap().item_index().unwrap(),
    )
}

#[test]
fn reading_map_base_is_readable_without_materialization_or_a_handle() {
    with_reading_map("North!", |generation, map, target| {
        let policy = generation
            .native_gameplay()
            .unwrap()
            .item_policy("oteryn:item.tibia.i2012", "definition-r1")
            .unwrap();
        assert!(!policy.record().admission.materializable);
        let facts = map.facts().pinned();
        let tile = view::compose_tile(
            map.overlay(),
            &facts,
            crate::map::overlay::TilePos {
                x: 7,
                y: 0,
                floor: -7,
            },
        )
        .unwrap()
        .unwrap();
        assert!(!tile.entries[1].source.handle_bearing());
        assert_eq!(
            facts.base_text(view::placement_key(tile.pos, 1).unwrap()),
            Some("North!")
        );
        for distance in 0..=4 {
            let at = ActorPosition {
                x: 7 - distance,
                y: 0,
                floor: -7,
            };
            assert_eq!(
                read_map_sign(generation, &map, &target, at),
                Ok(ItemLookResult::Text(
                    "You see sign.\nYou read: North!".into()
                ))
            );
        }
        let index = generation.native_gameplay().unwrap().item_index().unwrap();
        let reference = index
            .definition_ref("Item", "oteryn:item.tibia.i2012", "definition-r1")
            .unwrap();
        let definition = typed_definition(index, reference).unwrap();
        assert_eq!(
            examine_qualified_item(
                policy,
                ObservedLookItem {
                    item: ViewItem {
                        key: ItemKey::Instance(id(3)),
                        item_definition_ref: reference,
                        count: NonZeroU32::MIN,
                        sub_type: 0
                    },
                    definition: &definition,
                    look_distance: -1
                }
            ),
            ItemLookOutcome::Unavailable
        );
    });
}

#[test]
fn reading_map_five_and_visible_other_floor_are_too_far() {
    with_reading_map("North!", |generation, map, target| {
        for at in [
            ActorPosition {
                x: 2,
                y: 0,
                floor: -7,
            },
            ActorPosition {
                x: 7,
                y: 0,
                floor: -6,
            },
        ] {
            assert_eq!(
                read_map_sign(generation, &map, &target, at),
                Ok(ItemLookResult::Text(
                    "You see sign.\nYou are too far away to read it".into()
                ))
            );
        }
    });
}

#[test]
fn reading_map_empty_and_final_utf8_wire_bound() {
    for text in [String::new(), "界".repeat(200)] {
        with_reading_map(&text, |generation, map, target| {
            let Ok(ItemLookResult::Text(result)) = read_map_sign(
                generation,
                &map,
                &target,
                ActorPosition {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            ) else {
                panic!("visible sign");
            };
            if text.is_empty() {
                assert_eq!(result, "You see sign.\nNothing is written on it.");
            } else {
                let prefix = "You see sign.\nYou read: ";
                assert_eq!(
                    result,
                    format!(
                        "{prefix}{}",
                        "界".repeat((MAX_ITEM_LOOK_TEXT_BYTES - prefix.len()) / 3)
                    )
                );
                assert!(result.len() <= MAX_ITEM_LOOK_TEXT_BYTES);
                assert!(
                    oteryn_protocol_oteryn::item_look::encode_item_look_result(
                        &ItemLookResult::Text(result)
                    )
                    .is_ok()
                );
            }
        });
    }
}

#[test]
fn reading_map_forged_stale_hidden_and_not_visible_are_refused() {
    use crate::map::overlay::{Admission, TilePos};
    with_reading_map("North!", |generation, map, target| {
        let at = ActorPosition {
            x: 7,
            y: 0,
            floor: -7,
        };
        let mut forged = target.clone();
        forged.placement[0] ^= 1;
        assert_eq!(
            read_map_sign(generation, &map, &forged, at),
            Err(ItemLookRefusal::NotAvailable)
        );
        let mut stale = target.clone();
        stale.expected_revision = 1;
        assert_eq!(
            read_map_sign(generation, &map, &stale, at),
            Err(ItemLookRefusal::NotAvailable)
        );
        let mut missing = target.clone();
        missing.placement[39] = 63;
        assert_eq!(
            read_map_sign(generation, &map, &missing, at),
            Err(ItemLookRefusal::NotAvailable)
        );
        assert_eq!(
            read_map_sign(generation, &map, &target, ActorPosition { x: 30, ..at }),
            Err(ItemLookRefusal::NotVisible)
        );
        let mut overlay = crate::map::overlay::ChannelOverlay::new(
            std::sync::Arc::clone(map.base()),
            map.overlay().world_id(),
            map.overlay().channel_id(),
        );
        overlay
            .hide(
                TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
                1,
                Admission::Durable,
            )
            .unwrap();
        let source = super::super::world_map::MapViewSource {
            overlay: &overlay,
            facts: map.facts(),
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: 0,
        };
        assert_eq!(
            examine_world_object(
                &source,
                at,
                &target,
                generation,
                generation.native_gameplay().unwrap().item_index().unwrap()
            ),
            Err(ItemLookRefusal::NotAvailable)
        );
    });
}

#[test]
fn reading_map_placement_outside_the_published_stack_cut_is_not_visible() {
    with_reading_map_entries("North!", 10, |generation, map, target| {
        assert_eq!(
            read_map_sign(
                generation,
                &map,
                &target,
                ActorPosition {
                    x: 7,
                    y: 0,
                    floor: -7
                }
            ),
            Err(ItemLookRefusal::NotVisible)
        );
    });
}

#[test]
fn reading_map_non_distance_item_keeps_existing_look() {
    with_reading_map("North!", |generation, map, mut target| {
        let key = view::placement_key(
            crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
            1,
        )
        .unwrap();
        target.placement[32..].copy_from_slice(&key.to_be_bytes());
        let record = generation
            .native_gameplay()
            .unwrap()
            .item_policy("oteryn:item.tibia.i2854", "definition-r1")
            .unwrap()
            .record();
        let expected =
            oteryn_gameplay_items::look::item_text(&record.semantics, NonZeroU32::MIN, 1).unwrap();
        assert_eq!(
            read_map_sign(
                generation,
                &map,
                &target,
                ActorPosition {
                    x: 7,
                    y: 0,
                    floor: -7
                }
            ),
            Ok(ItemLookResult::Text(expected))
        );
        // A base ordinal naming a Terrain definition is not admitted as an Item.
        target.placement[39] = 0;
        assert_eq!(
            read_map_sign(
                generation,
                &map,
                &target,
                ActorPosition {
                    x: 7,
                    y: 0,
                    floor: -7
                }
            ),
            Err(ItemLookRefusal::NotAvailable)
        );
    });
}
