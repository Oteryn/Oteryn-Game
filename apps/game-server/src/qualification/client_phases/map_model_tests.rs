use super::*;

fn at(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

#[test]
fn map_journey_path_around_wall_and_no_path_through_blocking_item() -> TestResult {
    let open = |position: TilePos, direction| {
        let next = adjacent(position, direction)?;
        (next.x <= 4 && next.y <= 2 && next != at(2, 1, -7)).then_some(next)
    };
    let path = search(at(0, 1, -7), at(4, 1, -7), open, true).ok_or("wall detour missing")?;
    assert_eq!(path.len(), 6);
    assert!(search(at(0, 1, -7), at(2, 1, -7), open, true).is_none());
    Ok(())
}

#[test]
fn map_journey_synthetic_bundle_stairs_hole_collision_and_seeded_samples() -> TestResult {
    use crate::map::boot::{
        boot,
        tests::{floor_bundle, floor_items, floor_pins},
    };
    let (bytes, pins) = floor_bundle();
    let world = boot(
        &bytes,
        &floor_pins(pins),
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
        floor_items,
    )?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = MapModel::load(&world, &bytes, &root)?;
    let from = at(0, 0, -8);
    let upstairs = at(2, 0, -7);
    let path = model
        .path(from, upstairs, None)
        .ok_or("stair route missing")?;
    assert_eq!(path, [StepDirection::East]);
    assert_eq!(
        model.destination(upstairs, StepDirection::East),
        Some(at(3, 0, -8))
    );
    assert!(
        model.path(upstairs, at(1, 0, -7), None).is_none(),
        "solid box must remain unreachable"
    );
    assert!(!model.tiles[&at(1, 0, -7)].walkable);
    assert_eq!(model.samples(from, 123), model.samples(from, 123));
    assert!(
        model
            .samples(from, 123)
            .iter()
            .any(|sample| sample.kind == "stairs_up")
    );
    Ok(())
}

#[test]
fn map_journey_closed_door_blocks_without_key() -> TestResult {
    let world = crate::map::boot::tests::door_world()?;
    let door = crate::map::boot::tests::DOOR_AT;
    assert_eq!(
        destination(
            &world,
            at(door.x - 1, door.y, door.floor),
            StepDirection::East
        ),
        None
    );
    Ok(())
}

#[test]
fn map_journey_bundle_wall_detour_rope_and_unknown_ladder_facts() -> TestResult {
    use crate::map::boot::{
        boot,
        tests::{floor_bundle, floor_items, floor_pins},
    };
    use oteryn_world_bundle::{
        bundle::{Family, PaletteEntry},
        sector::{Attrs, Item, Tile},
    };
    let (bytes, mut pins) = floor_bundle();
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes)?;
    let rope_palette = bundle.manifest.palette.len() as u32;
    bundle.manifest.palette.push(PaletteEntry {
        key: "oteryn:terrain.tibia.i386".into(),
        family: Family::Terrain,
        id: 2,
        terrain: bundle.manifest.palette[0].terrain,
    });
    let ladder_palette = bundle.manifest.palette.len() as u32;
    bundle.manifest.palette.push(PaletteEntry {
        key: "oteryn:item.tibia.i1948".into(),
        family: Family::Item,
        id: 3,
        terrain: None,
    });
    bundle.sectors[0].tiles = (0..=2)
        .flat_map(|y| (0..=4).map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut tile = Tile {
                x,
                y,
                flags: u32::from(y == 0),
                house: if x == 4 { 7 } else { 0 },
                zones: Vec::new(),
                items: vec![Item {
                    palette: if (x, y) == (1, 2) { rope_palette } else { 0 },
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            };
            if (x, y) == (2, 1) {
                tile.items.push(Item {
                    palette: 3,
                    depth: 0,
                    attrs: Attrs::default(),
                });
            }
            if (x, y) == (3, 2) {
                tile.items.push(Item {
                    palette: ladder_palette,
                    depth: 0,
                    attrs: Attrs::default(),
                });
            }
            tile
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write_with_npcs(
        &bundle.manifest,
        &bundle.sectors,
        &bundle.spawns,
        &bundle.npcs,
    )?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
    let world = boot(
        &bytes,
        &floor_pins(pins),
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
        |key| {
            if key == "oteryn:item.tibia.i1948" {
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(4)?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            } else {
                floor_items(key)
            }
        },
    )?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = MapModel::load(&world, &bytes, &root)?;
    let from = at(0, 1, -8);
    let path = model
        .path(from, at(4, 1, -8), None)
        .ok_or("bundle wall detour missing")?;
    assert_eq!(path.len(), 6);
    assert!(model.path(from, at(2, 1, -8), None).is_none());
    assert!(model.tiles[&at(1, 2, -8)].rope_spot);
    assert_eq!(model.tiles[&at(1, 2, -8)].rope_ordinal, Some(0));
    assert!(model.tiles[&at(3, 2, -8)].ladder);
    assert!(
        !world.floors().contains(at(3, 2, -8)),
        "UNKNOWN ladder fact must not invent an automatic ascent"
    );
    assert_eq!(model.tiles[&at(4, 0, -8)].house, 7);
    assert!(model.tiles[&at(4, 0, -8)].protection_zone);
    let selected = model.samples(from, 1);
    for kind in selected.iter().map(|sample| sample.kind) {
        assert!(selected.iter().filter(|sample| sample.kind == kind).count() <= 5);
    }
    assert!(selected.iter().any(|sample| sample.kind == "rope_spot"));
    assert!(selected.iter().any(|sample| sample.kind == "ladder"));
    Ok(())
}
