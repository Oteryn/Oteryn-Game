use super::{TestResult, fixture::{debug, id}};
use crate::foundation::MovementLocalPosition;
use crate::map::{boot, overlay::TilePos};
use oteryn_world_bundle::{bundle, sector};

pub(super) fn local(at: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: at.x.into(),
        y: at.y.into(),
        floor: -i16::from(at.floor),
    }
}

pub(super) fn position(at: TilePos) -> serde_json::Value {
    serde_json::json!({"x":at.x,"y":at.y,"floor":at.floor})
}

type SceneTile = (TilePos, Option<(u32, Option<u16>)>);

pub(super) fn scene_world(
    start: TilePos,
    appearances: &[(u16, bool)],
    records: &[SceneTile],
    action_tiles: &[(TilePos, u16, Option<TilePos>)],
) -> TestResult<boot::BundleWorld> {
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for &(appearance, walkable) in appearances {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Border,
                walkable: Some(walkable),
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let tile = |at: TilePos, extra| {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if let Some((palette, unique)) = extra {
            let action_tile = action_tiles.iter().find(|(source, _, _)| *source == at);
            items.push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
                    action: action_tile.map(|(_, action, _)| *action),
                    teleport: action_tile
                        .and_then(|(_, _, destination)| *destination)
                        .map(|destination| {
                            (
                                destination.x,
                                destination.y,
                                destination.floor.to_ne_bytes()[0],
                            )
                        }),
                    unique,
                    ..Default::default()
                },
            });
        }
        sector::Tile {
            x: at.x,
            y: at.y,
            flags: 0,
            house: 0,
            zones: vec![],
            items,
        }
    };
    let mut floors = std::collections::BTreeSet::new();
    let mut by_sector = std::collections::BTreeMap::<_, Vec<_>>::new();
    for &(at, extra) in records {
        floors.insert(at.floor);
        by_sector
            .entry((at.floor, at.x / 32, at.y / 32))
            .or_default()
            .push(tile(at, extra));
    }
    data.manifest.world = bundle::Extent {
        min_x: records
            .iter()
            .map(|(at, _)| at.x)
            .min()
            .ok_or("minimum x")?,
        min_y: records
            .iter()
            .map(|(at, _)| at.y)
            .min()
            .ok_or("minimum y")?,
        max_x: records
            .iter()
            .map(|(at, _)| at.x)
            .max()
            .ok_or("maximum x")?
            + 1,
        max_y: records
            .iter()
            .map(|(at, _)| at.y)
            .max()
            .ok_or("maximum y")?
            + 1,
        floors: floors.into_iter().collect(),
    };
    let mut sectors: Vec<_> = by_sector
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
