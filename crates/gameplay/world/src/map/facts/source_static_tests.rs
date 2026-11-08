use super::*;

fn surface(appearance: u16) -> Result<PaletteEntry, String> {
    let key = format!("oteryn:world-object.tibia.i{appearance}");
    let source = crate::content::qualified_static_surface(&key)
        .ok_or("actual pinned static projectile facts")?;
    Ok(PaletteEntry {
        id: crate::content::static_surface_definition_id(&key).ok_or("source map definition")?,
        key,
        family: Family::Terrain,
        terrain: Some(oteryn_world_bundle::bundle::Terrain {
            kind: TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: Some(source.blocks_projectile),
        }),
    })
}

#[test]
fn srx3_static_world_object_keeps_native_blocking_and_has_no_item_reference() -> Result<(), String>
{
    let id = crate::content::static_surface_definition_id("oteryn:world-object.tibia.i10649")
        .ok_or("actual technical definition ID")?;
    let source = crate::content::qualified_static_surface("oteryn:world-object.tibia.i10649")
        .ok_or("actual pinned static projectile facts")?;
    let entry = PaletteEntry {
        key: "oteryn:world-object.tibia.i10649".into(),
        family: Family::Terrain,
        id,
        terrain: Some(oteryn_world_bundle::bundle::Terrain {
            kind: TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: Some(source.blocks_projectile),
        }),
    };
    let item_resolver_called = std::cell::Cell::new(false);
    let facts = palette_facts(&entry, &|_| {
        item_resolver_called.set(true);
        None
    })
    .map_err(|error| format!("qualified source static surface: {error:?}"))?;
    assert!(
        !item_resolver_called.get(),
        "static object must never resolve as inventory Item"
    );
    assert_eq!(facts.facts.appearance_id, 10649);
    assert!(facts.solid, "real pinned source surface blocks movement");
    assert!(
        !facts.facts.blocks_projectile,
        "source blocking flags are independent"
    );
    assert_eq!(
        facts.facts.definition,
        MapDefinition::Terrain(NonZeroU32::new(id + 1).ok_or("reference")?)
    );
    assert!(!facts.facts.pickupable);
    Ok(())
}

#[test]
fn srx3_static_world_object_flame_transforms_collision_with_its_visible_state() -> Result<(), String>
{
    use crate::map::boot::tests as fixture;
    use oteryn_world_bundle::{bundle, writer};
    let (bytes, mut pins) = fixture::bundle();
    let mut source = bundle::read(&bytes).map_err(|error| format!("fixture map: {error:?}"))?;
    let ordinal = u32::try_from(source.manifest.palette.len())
        .map_err(|error| format!("bounded palette: {error:?}"))?;
    source
        .manifest
        .palette
        .extend([surface(43497)?, surface(43498)?]);
    let tile = source.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 1)
        .ok_or("grounded cell")?;
    tile.items.push(oteryn_world_bundle::sector::Item {
        palette: ordinal,
        depth: 0,
        attrs: Default::default(),
    });
    let bytes = writer::write(&source.manifest, &source.sectors, &Default::default())
        .map_err(|error| format!("source surface map: {error:?}"))?;
    pins.digest = bundle::read(&bytes)
        .map_err(|error| format!("written map: {error:?}"))?
        .digest;
    let base =
        crate::map::load(&bytes, &pins).map_err(|error| format!("actual map base: {error:?}"))?;
    let mut facts = BundleFacts::palette(&source.manifest, fixture::items)
        .map_err(|error| format!("source facts: {error:?}"))?;
    let mut blocked = BTreeSet::new();
    for sector in &source.sectors {
        facts.push(&source.manifest, sector, &mut blocked);
    }
    let at = TilePos {
        x: 1,
        y: 0,
        floor: -7,
    };
    let key = placement_key(at, 1).ok_or("actual flame placement")?;
    assert!(facts.quest_callback_blocked(&base, at, blocked.contains(&at)));
    assert!(facts.transform_movement_item(key, 43498));
    assert!(
        !facts.quest_callback_blocked(&base, at, blocked.contains(&at)),
        "Crystal Bakragore's real 43497→43498 flame must become passable together with its sprite"
    );
    assert!(facts.transform_movement_item(key, 43497));
    assert!(facts.quest_callback_blocked(&base, at, blocked.contains(&at)));
    Ok(())
}

#[test]
fn srx3_source_static_transform_and_action_share_one_current_object_revision() -> Result<(), String>
{
    use crate::map::boot::tests as fixture;
    use crate::map::view::MapFacts;
    use oteryn_world_bundle::{bundle, writer};
    let (bytes, mut pins) = fixture::bundle();
    let mut source = bundle::read(&bytes).map_err(|error| format!("fixture map: {error:?}"))?;
    let ordinal = u32::try_from(source.manifest.palette.len())
        .map_err(|error| format!("palette ordinal: {error:?}"))?;
    // Real source-published static states; this tests the map owner primitive,
    // independently of the closed script rule choosing its action number.
    source
        .manifest
        .palette
        .extend([surface(43497)?, surface(43498)?]);
    let tile = source.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == 1)
        .ok_or("grounded cell")?;
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
        .map_err(|error| format!("source states: {error:?}"))?;
    pins.digest = bundle::read(&bytes)
        .map_err(|error| format!("written map: {error:?}"))?
        .digest;
    let base =
        crate::map::load(&bytes, &pins).map_err(|error| format!("actual map base: {error:?}"))?;
    let mut facts = BundleFacts::palette(&source.manifest, fixture::items)
        .map_err(|error| format!("qualified facts: {error:?}"))?;
    let mut blocked = BTreeSet::new();
    for sector in &source.sectors {
        facts.push(&source.manifest, sector, &mut blocked);
    }
    let at = TilePos {
        x: 1,
        y: 0,
        floor: -7,
    };
    let key = placement_key(at, 1).ok_or("source placement")?;
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
        crate::foundation::WorldId::decode(&source_id(1))
            .map_err(|error| format!("actual World identity: {error:?}"))?,
        crate::foundation::ChannelId::decode(&source_id(2))
            .map_err(|error| format!("actual Channel identity: {error:?}"))?,
    );
    let current = crate::map::view::compose_tile(&overlay, &facts, at)
        .map_err(|error| format!("current source stack: {error:?}"))?
        .ok_or("existing source cell")?;
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
        .ok_or("same physical placement")?
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
    Ok(())
}
