//! Test-only parameterization of the existing bundle/boot fixture, shared by movement lanes.
use super::*;

pub fn world() -> BundleWorld {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::Sector;
    let mut sectors = std::collections::BTreeMap::new();
    for y in 0..128_u16 {
        for x in 0..128_u16 {
            sectors
                .entry((y / 32, x / 32))
                .or_insert_with(Vec::new)
                .push(Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                });
        }
    }
    let sectors = sectors
        .into_iter()
        .map(|((sy, sx), tiles)| Sector {
            floor: -7,
            sx,
            sy,
            tiles,
        })
        .collect();
    super::world_from_bundle(super::bundle(Some(sectors)))
}

pub fn cells(world_id: WorldId) -> NativeEntryMovementCells {
    let entry =
        crate::content::qualify_native_entry_room(world_id).expect("existing entry qualification");
    world()
        .movement_cells(entry.movement_cells())
        .expect("existing bundle movement adapter")
}
