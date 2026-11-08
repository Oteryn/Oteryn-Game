//! Source selector geometry on neutral connecting floor; no quest state seeded.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn torch_world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::super::death_ml_loss_composed::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let native_index = native.item_index().ok_or("native item index")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    // Real source ground identity is required for known safe-landing spell facts.
    data.manifest.palette[0] = bundle::PaletteEntry {
        key: "oteryn:item.tibia.i351".into(),
        family: bundle::Family::Item,
        id: native_index
            .definition_ref("Item", "oteryn:item.tibia.i351", "definition-r1")
            .ok_or("native ground")?
            .get()
            - 1,
        terrain: Some(bundle::Terrain {
            blocks_projectile: None,
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(130),
        }),
    };
    let appearances = [2772, 2773, 352, 4611, 5770, 4634, 4636, 2473, 3031];
    for appearance in &appearances {
        let ground = matches!(appearance, 352 | 4611 | 5770);
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:item.tibia.i{appearance}"),
            family: bundle::Family::Item,
            id: native_index
                .definition_ref(
                    "Item",
                    &format!("oteryn:item.tibia.i{appearance}"),
                    "definition-r1",
                )
                .ok_or("native reference")?
                .get()
                - 1,
            terrain: if ground {
                Some(bundle::Terrain {
                    blocks_projectile: None,
                    kind: bundle::TerrainKind::Ground,
                    walkable: Some(*appearance != 4611),
                    ground_speed: Some(match appearance {
                        4611 => 0,
                        5770 => 90,
                        _ => 130,
                    }),
                })
            } else if matches!(appearance, 4634 | 4636) {
                Some(bundle::Terrain {
                    blocks_projectile: None,
                    kind: bundle::TerrainKind::Border,
                    walkable: None,
                    ground_speed: None,
                })
            } else {
                None
            },
        });
    }
    let mut grouped = std::collections::BTreeMap::new();
    for floor in [-8] {
        for y in 32184..=32208 {
            for x in 32094..=32106 {
                let object = match (floor, x, y) {
                    (-8, 32098 | 32104, 32204) => Some(2772),
                    (-8, 32099 | 32101 | 32102, 32205) => Some(352),
                    (-8, 32100, 32205) => Some(4611),
                    (-8, 32097, 32187) => Some(2473),
                    _ => None,
                };
                let mut items = vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: Default::default(),
                }];
                if let Some(appearance) = object {
                    if matches!(appearance, 352 | 4611) {
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
                if y == 32205 && matches!(x, 32099 | 32101) {
                    let appearance = if x == 32099 { 4634 } else { 4636 };
                    items.push(sector::Item {
                        palette: u32::try_from(
                            appearances
                                .iter()
                                .position(|a| *a == appearance)
                                .ok_or("border")?
                                + 1,
                        )?,
                        depth: 0,
                        attrs: Default::default(),
                    });
                }
                if (x, y) == (32099, 32205) {
                    items.push(sector::Item {
                        palette: u32::try_from(
                            appearances.iter().position(|a| *a == 3031).ok_or("coin")? + 1,
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
    data.manifest.world = bundle::Extent {
        min_x: 32094,
        min_y: 32184,
        max_x: 32107,
        max_y: 32209,
        floors: vec![-8],
    };
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
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins { map_revision:crate::map::overlay::map_revision(&base),bundle:pins,start:TilePos {x:32096,y:32204,floor:-8} },crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?, |key| {
        use crate::content::{ReferenceItemField,ReferenceItemType};
        let revision=native_index.revision(key)?;
        let reference=native_index.definition_ref("Item",key,revision)?;
        let policy=native.item_policy(key,revision).map(|p|p.record());
        Some(crate::map::facts::ItemDefinition{reference,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,ReferenceItemField::Known(f) if f.pickupable==ReferenceItemField::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door)))})
    }).map_err(debug)
}
