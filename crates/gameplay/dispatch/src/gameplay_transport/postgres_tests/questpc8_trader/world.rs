//! Source NPC cells and the literal untagged package box on neutral connecting floor.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let index = native.item_index().ok_or("items")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0] = bundle::PaletteEntry {
        key: "oteryn:item.tibia.i351".into(),
        family: bundle::Family::Item,
        id: index
            .definition_ref("Item", "oteryn:item.tibia.i351", "definition-r1")
            .ok_or("ground")?
            .get()
            - 1,
        terrain: Some(bundle::Terrain {
            blocks_projectile: None,
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(130),
        }),
    };
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i2472".into(),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            blocks_projectile: None,
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    data.manifest.world = bundle::Extent {
        min_x: 32200,
        min_y: 31100,
        max_x: 33300,
        max_y: 32900,
        floors: vec![-9, -8, -7, -6],
    };
    let mut cells = std::collections::BTreeMap::new();
    let areas = [
        (32307, 32834, -7),
        (32207, 31155, -7),
        (32300, 32837, -7),
        (32577, 32753, -7),
        (33066, 32879, -6),
        (33235, 32483, -7),
        (33166, 31810, -6),
        (32328, 31782, -6),
        (33214, 31793, -6),
        (32657, 32189, -8),
        (32661, 32192, -8),
        (33238, 32482, -7),
        (32636, 31667, -8),
        (32664, 31894, -9),
    ];
    for (cx, cy, floor) in areas {
        for y in cy - 3..=cy + 3 {
            for x in cx - 3..=cx + 3 {
                cells.insert((floor, x, y), ());
            }
        }
    }
    let mut grouped = std::collections::BTreeMap::new();
    for ((floor, x, y), ()) in cells {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if (x, y, floor) == (32661, 32192, -8) {
            items.push(sector::Item {
                palette: 1,
                depth: 0,
                attrs: Default::default(),
            });
        }
        grouped
            .entry((floor, y / 32, x / 32))
            .or_insert_with(Vec::new)
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 32306,
                y: 32834,
                floor: -7,
            },
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            let reference = index.definition_ref("Item", key, index.revision(key)?)?;
            let policy = native
                .item_policy(key, index.revision(key)?)
                .map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy
                    .and_then(|p| p.attributes.blocks_projectile)
                    .unwrap_or(true),
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
}
