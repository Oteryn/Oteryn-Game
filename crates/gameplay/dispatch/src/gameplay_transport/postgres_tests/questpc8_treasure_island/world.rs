//! Actual ship, selected sand and native underground rope cells on neutral connecting floor.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let (controller, _) = super::super::death_ml_loss_composed::production_generation_with_pin(
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        "content-1",
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("native index")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    let appearances = [351, 231, 386, 615];
    for appearance in appearances {
        let key = format!("oteryn:item.tibia.i{appearance}");
        data.manifest.palette.push(bundle::PaletteEntry {
            id: index
                .definition_ref("Item", &key, "definition-r1")
                .ok_or("native ground")?
                .get()
                - 1,
            key,
            family: bundle::Family::Item,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: bundle::TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        });
    }
    let mut grouped = std::collections::BTreeMap::new();
    for (floor, min_x, max_x, min_y, max_y) in [
        (-7, 32124u16, 32133u16, 32911u16, 32936u16),
        (-8, 32124, 32128, 32932, 32937),
        (-7, 32344, 32351, 32854, 32861),
    ] {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let appearance = match (floor, x, y) {
                    (-7, 32126, 32934) => 231,
                    (-8, 32126, 32934) => 386,
                    _ => 351,
                };
                grouped
                    .entry((floor, y / 32, x / 32))
                    .or_insert_with(Vec::new)
                    .push(sector::Tile {
                        x,
                        y,
                        flags: 0,
                        house: 0,
                        zones: Vec::new(),
                        items: vec![sector::Item {
                            palette: u32::try_from(
                                appearances
                                    .iter()
                                    .position(|a| *a == appearance)
                                    .ok_or("appearance")?,
                            )?,
                            depth: 0,
                            attrs: Default::default(),
                        }],
                    });
            }
        }
    }
    data.manifest.world = bundle::Extent {
        min_x: 32124,
        min_y: 32854,
        max_x: 32352,
        max_y: 32938,
        floors: vec![-8, -7],
    };
    let sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 32130,
                y: 32913,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            use crate::content::{ReferenceItemField,ReferenceItemType};
            let revision=index.revision(key)?;
            let reference=index.definition_ref("Item",key,revision)?;
            let policy=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition {
                reference,
                solid:policy.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
                pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,ReferenceItemField::Known(f) if f.pickupable==ReferenceItemField::Known(true))),
                door:policy.is_some_and(|p|matches!(&p.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door))),
            })
        },
    )
    .map_err(debug)
}
