//! Test-only parameterization of the existing bundle/boot fixture, shared by movement lanes.
use super::*;

pub fn world() -> BundleWorld {
    rectangle((0, 0), (127, 127), -7)
}

/// The same real bundle fixture at donor coordinates. Every authored test tile
/// has ground and no house/PZ/collision; production geometry is never replaced.
pub fn rectangle(from: (u16, u16), to: (u16, u16), native_floor: i8) -> BundleWorld {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::Sector;
    let mut sectors = std::collections::BTreeMap::new();
    for y in from.1..=to.1 {
        for x in from.0..=to.0 {
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
            floor: native_floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let extent = oteryn_world_bundle::writer::Extent {
        min_x: from.0,
        min_y: from.1,
        max_x: to.0.saturating_add(1),
        max_y: to.1.saturating_add(1),
        floors: vec![native_floor],
    };
    super::world_from_bundle(super::bundle_with_extent(Some(sectors), Some(extent)))
}

pub fn cells(world_id: WorldId) -> NativeEntryMovementCells {
    let entry =
        crate::content::qualify_native_entry_room(world_id).expect("existing entry qualification");
    world()
        .movement_cells(entry.movement_cells())
        .expect("existing bundle movement adapter")
}
