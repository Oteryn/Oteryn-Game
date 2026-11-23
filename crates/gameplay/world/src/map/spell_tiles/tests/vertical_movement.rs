//! Real bundle boot fixture for the Levitate and Magic Rope dispatch regression.
use super::*;

pub fn world(world: WorldId, channel: ChannelId) -> BundleWorld {
    world_with_rope_spot(world, channel, 386, false)
}

pub fn world_with_rope_spot(
    world: WorldId,
    channel: ChannelId,
    appearance: u32,
    distant_target: bool,
) -> BundleWorld {
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
    let mut lower = sector(-8, &[(10, 10, 4)]);
    if distant_target {
        let mut distant = sector(-8, &[(12, 10, 0)]);
        distant.tiles[0].items.push(Item {
            palette: 5,
            depth: 0,
            attrs: Attrs::default(),
        });
        lower.tiles.append(&mut distant.tiles);
    }
    let spot = format!("oteryn:item.tibia.i{appearance}");
    let (bytes, bundle) = super::bundle_with_projectiles(
        Some(vec![
            lower,
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
        &spot,
        None,
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
            super::items(match key {
                "oteryn:item.tibia.i103" => super::KEYS[0],
                key if key == spot => super::KEYS[4],
                key => key,
            })
        },
    )
    .expect("vertical movement bundle boots")
}
