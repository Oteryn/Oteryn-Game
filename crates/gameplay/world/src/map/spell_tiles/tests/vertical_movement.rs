//! Real bundle boot fixture for the Levitate and Magic Rope dispatch regression.
use super::*;

pub fn world(world: WorldId, channel: ChannelId) -> BundleWorld {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{Extent, Sector};
    let sector = |floor, positions: &[(u16, u16, u32)]| Sector {
        floor,
        sx: 0,
        sy: 0,
        tiles: positions
            .iter()
            .map(|&(x, y, palette)| Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![Item {
                    palette,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            })
            .collect(),
    };
    let (bytes, bundle) = super::bundle_with_ground(
        Some(vec![
            sector(-8, &[(10, 10, 4)]),
            sector(-7, &[(10, 9, 0), (10, 11, 0)]),
            sector(-6, &[(10, 10, 0)]),
            sector(-5, &[(10, 9, 0)]),
        ]),
        Some(Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-8, -7, -6, -5],
        }),
        "oteryn:item.tibia.i103",
    );
    let revision = format!(
        "sha256:{}",
        bundle
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    boot(
        &bytes,
        &BootPins {
            bundle,
            map_revision: revision,
            start: TilePos {
                x: 10,
                y: 10,
                floor: -6,
            },
        },
        world,
        channel,
        |key| {
            super::items(if key == "oteryn:item.tibia.i103" {
                super::KEYS[0]
            } else {
                key
            })
        },
    )
    .expect("vertical movement bundle boots")
}
