//! Seasonal Thais sewing table uses a literal native small-table counterpart; no seasonal sewing table is retained.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let appearances = [2319];
    let native_index = native.item_index().ok_or("native item index")?;
    for appearance in appearances {
        let key = format!("oteryn:item.tibia.i{appearance}");
        let reference = native_index
            .definition_ref("Item", &key, "definition-r1")
            .ok_or("sewing table definition")?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key,
            family: bundle::Family::Item,
            id: reference.get() - 1,
            terrain: None,
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: 32354,
        min_y: 32229,
        max_x: 32365,
        max_y: 32240,
        floors: vec![-7],
    };
    let mut grouped = std::collections::BTreeMap::new();
    for y in 32229..=32239 {
        for x in 32354..=32364 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (x, y) == (32359, 32235) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: Default::default(),
                });
            }
            grouped
                .entry((-7, y / 32, x / 32))
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
                x: 32358,
                y: 32234,
                floor: -7,
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
