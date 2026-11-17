//! Physical Family Brooch chest and the two source NPC islands.
use super::{TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
pub(super) fn world(native: &crate::content::native_gameplay::NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
use oteryn_world_bundle::{bundle,sector};
use std::collections::BTreeMap;
let index=native.item_index().ok_or("item index")?;
let reference=|key:&str|index.definition_ref("Item",key,"definition-r1");
let(bytes,mut pins)=crate::map::boot::tests::bundle();
let mut data=bundle::read(&bytes)?;
data.manifest.palette.truncate(1);
data.manifest.palette.push(bundle::PaletteEntry{id:1,key:"oteryn:terrain.tibia.i2476".into(),family:bundle::Family::Terrain,terrain:Some(bundle::Terrain{blocks_projectile:None,kind:bundle::TerrainKind::Common,walkable:None,ground_speed:None})});
let mut cells=BTreeMap::new();
for (z,x0,x1,y0,y1) in [(-8,32245,32251,31864,31869),(-6,32188,32194,31955,31961)] {
 for y in y0..=y1 {for x in x0..=x1 {cells.insert((z,x,y),vec![sector::Item{palette:0,depth:0,attrs:Default::default()}]);}}
}
cells.get_mut(&(-8,32248,31866)).ok_or("brooch source cell")?.push(sector::Item{palette:1,depth:0,attrs:sector::Attrs{unique:Some(6055),..Default::default()}});
let start=Position{x:32247,y:31866,floor:-8};
    data.manifest.world = bundle::Extent {
        min_x: cells
            .keys()
            .map(|(_, x, _)| *x)
            .min()
            .ok_or("empty world")?,
        min_y: cells
            .keys()
            .map(|(_, _, y)| *y)
            .min()
            .ok_or("empty world")?,
        max_x: 1 + cells
            .keys()
            .map(|(_, x, _)| *x)
            .max()
            .ok_or("empty world")?,
        max_y: 1 + cells
            .keys()
            .map(|(_, _, y)| *y)
            .max()
            .ok_or("empty world")?,
        floors: cells
            .keys()
            .map(|(z, _, _)| *z)
            .collect::<std::collections::BTreeSet<_>>()
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
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{map_revision:crate::map::overlay::map_revision(&base),bundle:pins,start:crate::map::overlay::TilePos{x:start.x,y:start.y,floor:start.floor}},
      crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?,crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?,|key|{
        use crate::content::{ReferenceItemField,ReferenceItemType};
        let policy=native.item_policy(key,"definition-r1")?.record();Some(crate::map::facts::ItemDefinition{reference:reference(key)?,solid:policy.attributes.blocks_movement,blocks_projectile:policy.attributes.blocks_projectile.unwrap_or(true),pickupable:matches!(&policy.semantics.physical,ReferenceItemField::Known(p) if p.pickupable==ReferenceItemField::Known(true)),door:matches!(&policy.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door))})
      }).map_err(debug)
}
