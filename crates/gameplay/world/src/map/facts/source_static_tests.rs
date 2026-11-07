use super::*;

fn surface(appearance: u16) -> PaletteEntry {
    let key = format!("oteryn:world-object.tibia.i{appearance}");
    PaletteEntry {
        id: crate::content::static_surface_definition_id(&key).expect("source map definition"),
        key,
        family: Family::Terrain,
        terrain: Some(oteryn_world_bundle::bundle::Terrain {
            kind: TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    }
}

#[test]
fn srx3_static_world_object_keeps_native_blocking_and_has_no_item_reference() {
    let id = crate::content::static_surface_definition_id("oteryn:world-object.tibia.i10649")
        .expect("actual technical definition ID");
    let entry = PaletteEntry {
        key: "oteryn:world-object.tibia.i10649".into(),
        family: Family::Terrain,
        id,
        terrain: Some(oteryn_world_bundle::bundle::Terrain {
            kind: TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    };
    let facts = palette_facts(&entry, &|_| {
        panic!("static object must never resolve as inventory Item")
    })
    .expect("qualified source static surface");
    assert_eq!(facts.facts.appearance_id, 10649);
    assert!(facts.solid, "real pinned source surface blocks movement");
    assert!(
        !facts.facts.blocks_projectile,
        "source blocking flags are independent"
    );
    assert_eq!(
        facts.facts.definition,
        MapDefinition::Terrain(NonZeroU32::new(id + 1).expect("reference"))
    );
    assert!(!facts.facts.pickupable);
}

#[test]
fn srx3_static_world_object_flame_transforms_collision_with_its_visible_state() {
    use crate::map::boot::tests as fixture;
    use oteryn_world_bundle::{bundle, writer};
    let (bytes, mut pins) = fixture::bundle();
    let mut source = bundle::read(&bytes).expect("fixture map");
    let ordinal = u32::try_from(source.manifest.palette.len()).expect("bounded palette");
    source
        .manifest
        .palette
        .extend([surface(43497), surface(43498)]);
    let tile = source.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 1)
        .expect("grounded cell");
    tile.items.push(oteryn_world_bundle::sector::Item {
        palette: ordinal,
        depth: 0,
        attrs: Default::default(),
    });
    let bytes = writer::write(&source.manifest, &source.sectors, &Default::default())
        .expect("source surface map");
    pins.digest = bundle::read(&bytes).expect("written map").digest;
    let base = crate::map::load(&bytes, &pins).expect("actual map base");
    let mut facts = BundleFacts::palette(&source.manifest, fixture::items).expect("source facts");
    let mut blocked = BTreeSet::new();
    for sector in &source.sectors {
        facts.push(&source.manifest, sector, &mut blocked);
    }
    let at = TilePos {
        x: 1,
        y: 0,
        floor: -7,
    };
    let key = placement_key(at, 1).expect("actual flame placement");
    assert!(facts.quest_callback_blocked(&base, at, blocked.contains(&at)));
    assert!(facts.transform_movement_item(key, 43498));
    assert!(
        !facts.quest_callback_blocked(&base, at, blocked.contains(&at)),
        "Crystal Bakragore's real 43497→43498 flame must become passable together with its sprite"
    );
    assert!(facts.transform_movement_item(key, 43497));
    assert!(facts.quest_callback_blocked(&base, at, blocked.contains(&at)));
}

#[test]
fn srx3_source_static_transform_and_action_share_one_current_object_revision() {
    use crate::map::boot::tests as fixture;
    use crate::map::view::MapFacts;
    use oteryn_world_bundle::{bundle, writer};
    let (bytes, mut pins) = fixture::bundle();
    let mut source = bundle::read(&bytes).expect("fixture map");
    let ordinal = u32::try_from(source.manifest.palette.len()).expect("palette ordinal");
    // Real source-published static states; this tests the map owner primitive,
    // independently of the closed script rule choosing its action number.
    source
        .manifest
        .palette
        .extend([surface(43497), surface(43498)]);
    let tile = source.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 1)
        .expect("grounded cell");
    tile.items.push(oteryn_world_bundle::sector::Item {
        palette: ordinal,
        depth: 0,
        attrs: oteryn_world_bundle::sector::Attrs {
            action: Some(14350),
            unique: Some(50001),
            description: Some("retained source placement".into()),
            ..Default::default()
        },
    });
    let bytes = writer::write(&source.manifest, &source.sectors, &Default::default())
        .expect("source states");
    pins.digest = bundle::read(&bytes).expect("written map").digest;
    let base = crate::map::load(&bytes, &pins).expect("actual map base");
    let mut facts =
        BundleFacts::palette(&source.manifest, fixture::items).expect("qualified facts");
    let mut blocked = BTreeSet::new();
    for sector in &source.sectors {
        facts.push(&source.manifest, sector, &mut blocked);
    }
    let at = TilePos {
        x: 1,
        y: 0,
        floor: -7,
    };
    let key = placement_key(at, 1).expect("source placement");
    assert_eq!(facts.action_at(key), Some(14350));
    assert!(facts.transform_movement_item_with_action(key, 0, 43497, 43498, Some(14325)));
    assert_eq!(
        facts.action_at(key),
        Some(14325),
        "current source action changes with the semantic object state"
    );
    assert_eq!(facts.object_revision(key), 1);
    assert!(!facts.quest_callback_blocked(&base, at, blocked.contains(&at)));
    let source_id = |tag| [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let overlay = crate::map::overlay::ChannelOverlay::new(
        Arc::new(base),
        crate::foundation::WorldId::decode(&source_id(1)).expect("actual World identity"),
        crate::foundation::ChannelId::decode(&source_id(2)).expect("actual Channel identity"),
    );
    let current = crate::map::view::compose_tile(&overlay, &facts, at)
        .expect("current source stack")
        .expect("existing source cell");
    assert!(
        current
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == 43498)
    );
    let original = &facts
        .placements
        .iter()
        .find(|(placement, _)| *placement == key)
        .expect("same physical placement")
        .1
        .attrs;
    assert_eq!(original.unique, Some(50001));
    assert_eq!(
        original.description.as_deref(),
        Some("retained source placement")
    );
    assert!(
        !facts.transform_movement_item_with_action(key, 0, 43497, 43498, Some(14325)),
        "stale revision cannot publish a second transition"
    );
    assert!(
        !facts.transform_movement_item_with_action(key, 1, 43497, 43497, Some(14350)),
        "wrong current source state refuses before mutation"
    );
    assert_eq!(facts.action_at(key), Some(14325));
    assert_eq!(facts.object_revision(key), 1);
    assert!(facts.transform_movement_item_with_action(key, 1, 43498, 43497, Some(14350)));
    assert_eq!(facts.action_at(key), Some(14350));
    assert_eq!(facts.object_revision(key), 2);
    assert!(facts.quest_callback_blocked(overlay.base(), at, blocked.contains(&at)));
}
