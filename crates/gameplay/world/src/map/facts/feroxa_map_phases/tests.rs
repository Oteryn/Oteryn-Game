use super::*;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::content::{CollisionClass, NativeSpellTileFlags};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_world_bundle::{bundle, sector};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(n: u8) -> [u8; 16] {
    let mut value = [n; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

#[test]
fn qn5_feroxa_phase_publishes_ground_collision_combat_and_pinned_views_atomically() -> TestResult {
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i13988".into();
    data.manifest.palette[0]
        .terrain
        .as_mut()
        .ok_or("ground")?
        .walkable = Some(false);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i20324".into(),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
            blocks_projectile: Some(false),
        }),
    });
    let start = TilePos {
        x: 33414,
        y: 31522,
        floor: -11,
    };
    let at = TilePos { x: 33415, ..start };
    let added = TilePos { y: at.y + 1, ..at };
    data.manifest.world = bundle::Extent {
        min_x: start.x,
        min_y: start.y,
        max_x: at.x + 1,
        max_y: added.y + 1,
        floors: vec![-11],
    };
    let tile = |p: TilePos, palette| sector::Tile {
        x: p.x,
        y: p.y,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![sector::Item {
            palette,
            depth: 0,
            attrs: Default::default(),
        }],
    };
    let sectors = vec![bundle::Sector {
        floor: -11,
        sx: start.x / 32,
        sy: start.y / 32,
        tiles: vec![tile(start, 1), tile(at, 0)],
    }];
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(|e| format!("{e:?}"))?;
    let world = crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start,
        },
        world_id,
        crate::foundation::ChannelId::decode(&id(43)).map_err(|e| format!("{e:?}"))?,
        |_| None,
    )?;
    let room = crate::content::qualify_native_entry_room(world_id).map_err(|e| format!("{e:?}"))?;
    let cells = world
        .movement_cells(room.movement_cells())
        .map_err(|e| format!("{e:?}"))?;
    let logical = |p: TilePos| LogicalCell {
        x: i32::from(p.x),
        y: i32::from(p.y),
        z: -i32::from(p.floor),
    };
    let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
        return Err("bundle".into());
    };
    let facts = world.facts();
    let before = facts.pinned();
    assert!(!world.enterable(at));
    let phase_tiles = |appearance, blocked, no_logout| -> Option<Vec<FeroxaPhaseTile>> {
        let entry = facts.movement_item_facts(appearance)?;
        Some(
            [at, added]
                .into_iter()
                .map(|position| FeroxaPhaseTile {
                    position,
                    entries: vec![entry],
                    no_logout,
                    teleport: None,
                    spell: QualifiedSpellTile::from_bundle(
                        Some((u32::from(appearance), 150)),
                        vec![],
                        NativeSpellTileFlags {
                            block_solid: blocked,
                            block_projectile: false,
                            immovable_block_solid: blocked,
                            immovable_block_item: false,
                            immovable_nonfield_block_item: false,
                            floor_change: false,
                            protection_zone: false,
                        },
                        Some((false, false)),
                    ),
                })
                .collect(),
        )
    };
    let middle = phase_tiles(20324, false, true).ok_or("middle")?;
    assert!(facts.publish_feroxa_phase(id(91), 1, [2; 32], middle.clone()));
    assert!(
        world.enterable(at),
        "the actual movement owner must read the new phase ground"
    );
    assert_eq!(
        index.lookup(cells.scope(), logical(added)),
        Ok(CollisionClass::Walkable)
    );
    let rendered = view::compose_tile(world.overlay(), &facts.pinned(), at)
        .map_err(|e| format!("{e:?}"))?
        .ok_or("phase view")?;
    assert_eq!(rendered.entries.len(), 1);
    assert_eq!(rendered.entries[0].facts.appearance_id, 20324);
    assert_eq!(
        view::compose_tile(world.overlay(), &before, at)
            .map_err(|e| format!("{e:?}"))?
            .ok_or("old view")?
            .entries[0]
            .facts
            .appearance_id,
        13988
    );
    let first = cells
        .spell_tiles()
        .lookup(cells.scope(), logical(at))
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(first.ground_source_id(), Some(20324));
    assert!(!first.flags().block_solid);
    assert_eq!(
        index.map().zone_flags(at),
        Some((true, false)),
        "logout reads the same phase flags"
    );
    let revision = facts.object_revision(placement_key(at, 0).ok_or("key")?);
    assert!(facts.publish_feroxa_phase(id(91), 1, [2; 32], middle.clone()));
    assert_eq!(
        facts.object_revision(placement_key(at, 0).ok_or("key")?),
        revision
    );
    let fork = index
        .fork_activity(
            crate::foundation::RuntimeScopeRefV1::instance(world_id, id(92))
                .map_err(|e| format!("{e:?}"))?,
            [33414, 31522, 33415, 31523, 11, 11],
        )
        .ok_or("private map fork")?;
    assert_eq!(
        fork.map()
            .spell_tiles
            .lookup(logical(at))
            .map_err(|e| format!("{e:?}"))?
            .ground_source_id(),
        Some(13988),
        "a private map must not inherit its parent's live phase owner"
    );
    let key = placement_key(at, 0).ok_or("phase key")?;
    assert!(
        !facts.apply_quest_callback(
            key,
            0,
            facts.movement_item_facts(13988).ok_or("blocked ground")?,
            &[],
            &[],
            false,
            None
        ),
        "pre-phase revisions cannot mutate current scenery"
    );
    assert!(facts.apply_quest_callback(
        key,
        revision,
        facts.movement_item_facts(13988).ok_or("blocked ground")?,
        &[],
        &[],
        false,
        None
    ));
    assert_eq!(
        cells.spell_tiles().lookup(cells.scope(), logical(at)),
        Err(SpellTileLookupError::Unknown),
        "an unsupported later transform cannot reuse old phase combat flags"
    );
    assert!(!world.enterable(at));
    let final_map = phase_tiles(13988, true, false).ok_or("final")?;
    assert!(facts.publish_feroxa_phase(id(91), 2, [3; 32], final_map.clone()));
    assert!(!world.enterable(at));
    assert!(
        !first.flags().block_solid,
        "a prior combat read remains pinned"
    );
    assert!(facts.object_revision(placement_key(at, 0).ok_or("key")?) > revision);
    assert!(!facts.publish_feroxa_phase(id(91), 1, [2; 32], middle));
    assert!(!facts.publish_feroxa_phase(id(93), 2, [3; 32], final_map.clone()));
    let mut substituted = final_map.clone();
    substituted[0].no_logout = !substituted[0].no_logout;
    assert!(
        !facts.publish_feroxa_phase(id(91), 2, [3; 32], substituted),
        "the same source digest cannot substitute a different phase payload"
    );
    let mut portable = final_map.clone();
    portable[0].entries[0].pickupable = true;
    assert!(
        !facts.publish_feroxa_phase(id(91), 2, [3; 32], portable),
        "scenery publication cannot acquire portable custody"
    );
    assert!(!facts.publish_feroxa_phase(id(91), 2, [4; 32], final_map));
    Ok(())
}

#[test]
fn qn5_feroxa_retained_maps_prepare_complete_native_stacks_and_refuse_missing_definitions()
-> TestResult {
    let resolve = |id| {
        Some(EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::new(u32::from(id))?),
            terrain_kind: None,
            appearance_id: id,
            blocks_projectile: false,
            pickupable: true,
            bound: false,
            count: 99,
            sub_type: 7,
        })
    };
    assert!(prepare_feroxa_phase(1_799_999, resolve).is_none());
    let (middle, digest, before) = prepare_feroxa_phase(1_800_000, resolve).ok_or("middle")?;
    let (final_map, final_digest, after) =
        prepare_feroxa_phase(3_600_000, resolve).ok_or("final")?;
    assert_eq!((middle, final_map), (1, 2));
    assert_eq!(digest, final_digest);
    assert_ne!(digest, [0; 32]);
    assert_eq!((before.len(), after.len()), (1225, 1225));
    assert_eq!(
        before.iter().zip(&after).filter(|(a, b)| a != b).count(),
        806
    );
    assert_eq!(before.iter().filter(|t| t.no_logout).count(), 1056);
    assert_eq!(after.iter().filter(|t| t.no_logout).count(), 507);
    for tiles in [&before, &after] {
        let portals: Vec<_> = tiles.iter().filter(|t| t.teleport.is_some()).collect();
        assert_eq!(portals.len(), 1);
        assert_eq!(
            portals[0].position,
            TilePos {
                x: 33430,
                y: 31537,
                floor: -11
            }
        );
        assert_eq!(
            portals[0].teleport,
            Some((
                1,
                TilePos {
                    x: 33457,
                    y: 31531,
                    floor: -11
                }
            ))
        );
        assert!(portals[0].entries[1].bound);
        assert_eq!(
            tiles.iter().filter(|t| t.spell.flags().block_solid).count(),
            729
        );
        assert_eq!(
            tiles
                .iter()
                .filter(|t| t.spell.flags().block_projectile)
                .count(),
            729
        );
        for tile in tiles {
            assert!(!tile.spell.flags().protection_zone);
            for entry in &tile.entries {
                assert!(!entry.pickupable);
                assert_eq!((entry.count, entry.sub_type), (1, 0));
            }
        }
    }
    assert!(
        prepare_feroxa_phase(1_800_000, |id| if id == 1949 { None } else { resolve(id) }).is_none()
    );
    assert!(
        prepare_feroxa_phase(3_600_000, |id| resolve(id).map(|mut entry| {
            entry.appearance_id = 0;
            entry
        }))
        .is_none()
    );
    Ok(())
}
