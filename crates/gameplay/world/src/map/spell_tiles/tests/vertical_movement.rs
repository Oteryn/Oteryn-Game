//! Real bundle boot fixture for the Levitate and Magic Rope dispatch regression.
use super::*;

pub fn world(world: WorldId, channel: ChannelId) -> BundleWorld {
    world_with_rope_spot(world, channel, 386, false, false, false)
}

pub fn world_with_rope_spot(
    world: WorldId,
    channel: ChannelId,
    appearance: u32,
    distant_target: bool,
    covered_spot: bool,
    gate: bool,
) -> BundleWorld {
    vertical_world(
        world,
        channel,
        appearance,
        distant_target,
        covered_spot,
        gate,
        false,
    )
}

/// A Crystal i385 open hole above regular ground, with a south landing.
pub fn world_with_rope_hole(world: WorldId, channel: ChannelId) -> BundleWorld {
    vertical_world(world, channel, 385, false, false, false, true)
}

fn vertical_world(
    world: WorldId,
    channel: ChannelId,
    appearance: u32,
    distant_target: bool,
    covered_spot: bool,
    gate: bool,
    hole: bool,
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
    let mut lower = if hole {
        sector(-8, &[(11, 10, 0)])
    } else {
        sector(-8, &[(10, 10, 4)])
    };
    let upstairs = if hole {
        sector(-7, &[(10, 10, 0), (11, 10, 4), (11, 11, 0)])
    } else {
        sector(-7, &[(10, 9, 0), (10, 11, 0)])
    };
    if gate {
        lower.tiles[0].items.push(Item {
            palette: 8,
            depth: 0,
            attrs: Attrs {
                action: Some(1001),
                ..Attrs::default()
            },
        });
        lower.tiles.append(&mut sector(-8, &[(11, 10, 0)]).tiles);
    }
    if covered_spot {
        lower.tiles[0].items.push(Item {
            palette: 5,
            depth: 0,
            attrs: Attrs::default(),
        });
    }
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
    let (mut bytes, mut bundle) = super::bundle_with_projectiles(
        Some(vec![
            lower,
            upstairs,
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
    if gate {
        let mut data = oteryn_world_bundle::bundle::read(&bytes).expect("fixture bundle");
        data.manifest.palette[8].key = "oteryn:item.tibia.i1648".into();
        bytes = oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &data.spawns)
            .expect("gated fixture bundle");
        bundle.digest = oteryn_world_bundle::bundle::read(&bytes)
            .expect("gated bundle")
            .digest;
    }
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
            if gate && matches!(key, "oteryn:item.tibia.i1648" | "oteryn:item.tibia.i1649") {
                let closed = key.ends_with("1648");
                return Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(if closed { 9 } else { 1649 })?,
                    solid: Some(closed),
                    blocks_projectile: closed,
                    pickupable: false,
                    door: true,
                });
            }
            super::items(match key {
                "oteryn:item.tibia.i103" => super::KEYS[0],
                key if key == spot => super::KEYS[4],
                key => key,
            })
        },
    )
    .expect("vertical movement bundle boots")
}
