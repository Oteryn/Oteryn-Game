//! Compact journey map: native quest door, source selector and adjacent chest.
//! NPC deployment provenance is checked separately by the placement producer.
use super::TestResult;
use super::fixture::{debug, id};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_world_bundle::{bundle, sector};
use std::collections::BTreeMap;

pub(super) fn world(
    native: &NativeGameplayState,
    sam: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (x, y, floor, action, unique, appearance) = if sam {
        (32455_u16, 31967_u16, -14_i8, 40603_u16, 9226_u16, 2472_u16)
    } else {
        (32467, 31969, -5, 41492, 6257, 2469)
    };
    let index = native.item_index().ok_or("native Item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let door = "oteryn:item.tibia.i5113";
    let coin = "oteryn:item.tibia.i3031";
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (door, reference(door).ok_or("native quest door")?.get() - 1),
        (coin, reference(coin).ok_or("native coin")?.get() - 1),
    );
    let mut data = bundle::read(&bytes)?;
    let door_palette = u32::try_from(
        data.manifest
            .palette
            .iter()
            .position(|p| p.key == door)
            .ok_or("door palette")?,
    )?;
    let chest_palette = u32::try_from(data.manifest.palette.len())?;
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{appearance}"),
        family: bundle::Family::Terrain,
        id: chest_palette,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    let mut groups = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for tx in x - 2..=x + 3 {
        for ty in y - 3..=y + 2 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (tx, ty) == (x, y) {
                items.push(sector::Item {
                    palette: door_palette,
                    depth: 0,
                    attrs: sector::Attrs {
                        action: Some(action),
                        ..Default::default()
                    },
                });
            } else if (tx, ty) == (x, y + 1) {
                items.push(sector::Item {
                    palette: chest_palette,
                    depth: 0,
                    attrs: sector::Attrs {
                        unique: Some(unique),
                        ..Default::default()
                    },
                });
            }
            groups
                .entry((floor, tx / 32, ty / 32))
                .or_default()
                .push(sector::Tile {
                    x: tx,
                    y: ty,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
    }
    data.manifest.world = bundle::Extent {
        min_x: x - 2,
        min_y: y - 3,
        max_x: x + 4,
        max_y: y + 3,
        floors: vec![floor],
    };
    let sectors: Vec<_> = groups
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
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: x - 1,
                y: y - 1,
                floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            use crate::content::{ReferenceItemField, ReferenceItemType};
            let policy = native.item_policy(key, "definition-r1")?.record();
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: matches!(&policy.semantics.physical,
                    ReferenceItemField::Known(p) if p.pickupable == ReferenceItemField::Known(true)),
                door: matches!(&policy.semantics.classification,
                    ReferenceItemField::Known(c) if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door)),
            })
        },
    )
    .map_err(debug)
}
