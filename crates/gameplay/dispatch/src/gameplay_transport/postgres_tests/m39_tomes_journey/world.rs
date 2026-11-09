//! Source chest cells connected by fixture-only portals; no book or progress is seeded.
use super::*;
use oteryn_world_bundle::{bundle, sector};
use std::collections::{BTreeMap, BTreeSet};

fn books() -> TestResult<Vec<(Position, u16)>> {
    let mut books = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-callback/l2-tome-"))
        .map(|c| {
            Ok((
                c.selector.position.ok_or("book position")?,
                c.selector.appearance.ok_or("book appearance")?,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    books.sort_by_key(|(p, _)| (p.floor, p.x, p.y));
    assert_eq!(books.len(), 10);
    Ok(books)
}
pub(crate) fn start() -> TestResult<Position> {
    let p = books()?.first().ok_or("source books")?.0;
    Ok(Position {
        x: p.x - 1,
        y: p.y,
        floor: p.floor,
    })
}
pub(crate) fn build(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let books = books()?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for (n, appearance) in [2472, 2480].into_iter().enumerate() {
        data.manifest.palette.push(bundle::PaletteEntry {
            id: 100 + u32::try_from(n)?,
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let index = native.item_index().ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    data.manifest.palette.push(bundle::PaletteEntry {
        id: reference("oteryn:item.tibia.i1949")
            .ok_or("native portal")?
            .get()
            - 1,
        key: "oteryn:item.tibia.i1949".into(),
        family: bundle::Family::Item,
        terrain: None,
    });
    let mut cells = BTreeMap::<(i8, u16, u16), Vec<sector::Item>>::new();
    for (p, _) in &books {
        for x in p.x - 2..=p.x + 2 {
            for y in p.y - 2..=p.y + 2 {
                cells.entry((p.floor, x, y)).or_insert_with(|| {
                    vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: Default::default(),
                    }]
                });
            }
        }
    }
    for (n, (p, appearance)) in books.iter().enumerate() {
        cells
            .get_mut(&(p.floor, p.x, p.y))
            .ok_or("chest cell")?
            .push(sector::Item {
                palette: if *appearance == 2472 { 1 } else { 2 },
                depth: 0,
                attrs: Default::default(),
            });
        let next = books[(n + 1) % books.len()].0;
        cells
            .get_mut(&(p.floor, p.x - 1, p.y + 1))
            .ok_or("portal cell")?
            .push(sector::Item {
                palette: 3,
                depth: 0,
                attrs: sector::Attrs {
                    teleport: Some((next.x - 1, next.y, next.floor.to_le_bytes()[0])),
                    ..Default::default()
                },
            });
    }
    data.manifest.world = bundle::Extent {
        min_x: cells.keys().map(|(_, x, _)| *x).min().ok_or("world")?,
        min_y: cells.keys().map(|(_, _, y)| *y).min().ok_or("world")?,
        max_x: 1 + cells.keys().map(|(_, x, _)| *x).max().ok_or("world")?,
        max_y: 1 + cells.keys().map(|(_, _, y)| *y).max().ok_or("world")?,
        floors: cells
            .keys()
            .map(|(z, _, _)| *z)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    let mut grouped = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for ((z, x, y), items) in cells {
        grouped
            .entry((z, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors = Vec::new();
    for ((floor, sx, sy), mut tiles) in grouped {
        tiles.sort_by_key(|t| (t.y, t.x));
        sectors.push(bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        });
    }
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let start = start()?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins {
        map_revision:crate::map::overlay::map_revision(&base),bundle:pins,
        start:crate::map::overlay::TilePos{x:start.x,y:start.y,floor:start.floor}},
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key| {
            use crate::content::{ReferenceItemField,ReferenceItemType};
            let p=native.item_policy(key,"definition-r1")?.record();
            Some(crate::map::facts::ItemDefinition{reference:reference(key)?,solid:p.attributes.blocks_movement,
                blocks_projectile:p.attributes.blocks_projectile.unwrap_or(true),
                pickupable:matches!(&p.semantics.physical,ReferenceItemField::Known(x) if x.pickupable==ReferenceItemField::Known(true)),
                door:matches!(&p.semantics.classification,ReferenceItemField::Known(x) if x.item_type==ReferenceItemField::Known(ReferenceItemType::Door))})
        }).map_err(debug)
}
