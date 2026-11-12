//! Neutral connecting floor retains the native hole, ladder, door, fires and cache selectors.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let appearances = [432, 1948, 1631, 2472, 2123, 1633];
    let native_index = native.item_index().ok_or("native item index")?;
    for (index, appearance) in appearances.iter().enumerate() {
        if matches!(*appearance, 1631 | 1633) {
            let key = format!("oteryn:item.tibia.i{appearance}");
            let reference = native_index
                .definition_ref("Item", &key, "definition-r1")
                .ok_or("door reference")?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key,
                family: bundle::Family::Item,
                id: reference.get() - 1,
                terrain: None,
            });
            continue;
        }
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(index + 1)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if *appearance == 432 {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: if *appearance == 432 { Some(true) } else { None },
                ground_speed: if *appearance == 432 { Some(150) } else { None },
            }),
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: 32569,
        min_y: 32190,
        max_x: 32583,
        max_y: 32203,
        floors: vec![-15, -14],
    };
    let mut grouped = std::collections::BTreeMap::new();
    for floor in [-15, -14] {
        for y in 32190..=32202 {
            for x in 32569..=32582 {
                let object = match (floor, x, y) {
                    (-14, 32576, 32198) => Some(432),
                    (-15, 32576, 32198) => Some(1948),
                    (-15, 32578, 32197) => Some(1631),
                    (-15, 32577 | 32578, 32193) => Some(2472),
                    (-15, 32576 | 32580, 32196) => Some(2123),
                    _ => None,
                };
                let mut items = vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: Default::default(),
                }];
                if let Some(appearance) = object {
                    if appearance == 432 {
                        items.clear();
                    }
                    items.push(sector::Item {
                        palette: u32::try_from(
                            appearances
                                .iter()
                                .position(|a| *a == appearance)
                                .ok_or("appearance")?
                                + 1,
                        )?,
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
        }
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
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
                x: 32574,
                y: 32197,
                floor: -14,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            use crate::content::{ReferenceItemField,ReferenceItemType};
            let revision=native_index.revision(key)?;
            let reference=native_index.definition_ref("Item",key,revision)?;
            let policy=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition {
                reference,solid:policy.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
                pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,ReferenceItemField::Known(f) if f.pickupable==ReferenceItemField::Known(true))),
                door:policy.is_some_and(|p|matches!(&p.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door))),
            })
        },
    )
    .map_err(debug)
}
