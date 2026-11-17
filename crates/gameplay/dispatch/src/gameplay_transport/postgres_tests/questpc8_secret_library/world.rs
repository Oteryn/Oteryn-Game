//! Small disjoint native objective areas. Travel is fixture-only; objectives retain their coordinates.
use super::*;
use oteryn_world_bundle::{bundle, sector};

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    for (index, appearance) in [351, 4527, 23881, 25051].into_iter().enumerate() {
        let ground = appearance != 25051;
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(index)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if ground {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: ground.then_some(true),
                ground_speed: ground.then_some(150),
            }),
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: 32028,
        min_y: 31311,
        max_x: 33529,
        max_y: 32790,
        floors: vec![-15, -14, -11, -10, -9, -7, -6, -1],
    };
    let areas = [
        (-7, 32028, 32460, 32034, 32466),
        (-6, 32526, 32782, 32532, 32788),
        (-1, 33265, 31830, 33271, 31836),
        (-14, 33522, 31461, 33528, 31467),
        (-10, 32804, 32761, 32812, 32769),
        (-6, 32618, 32735, 32624, 32741),
        (-9, 33356, 31311, 33376, 31328),
        (-15, 32477, 32594, 32483, 32600),
        (-11, 32669, 32733, 32675, 32739),
    ];
    let mut grouped = std::collections::BTreeMap::new();
    for (floor, min_x, min_y, max_x, max_y) in areas {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let ground = match (floor, x, y) {
                    (-7, 32031, 32463) => 1,
                    (-10, 32807, 32765) => 2,
                    _ => 0,
                };
                let mut items = vec![sector::Item {
                    palette: ground,
                    depth: 0,
                    attrs: Default::default(),
                }];
                if (floor, x, y) == (-10, 32808, 32765) {
                    items.push(sector::Item {
                        palette: 3,
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
    let sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sy,
            sx,
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
                x: 32030,
                y: 32463,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
