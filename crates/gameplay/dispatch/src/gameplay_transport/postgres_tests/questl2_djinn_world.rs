//! Physical UID/AID fixtures from Crystal quest_system2 and the Djinn fountain action.
use super::{TestResult, fixture::debug};

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        id: 1,
        key: "oteryn:terrain.tibia.i3230".into(),
        family: bundle::Family::Terrain,
        terrain: Some(bundle::Terrain {
            blocks_projectile: None,
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    data.manifest.world = bundle::Extent {
        min_x: 33288,
        min_y: 31673,
        max_x: 33300,
        max_y: 31681,
        floors: vec![-15],
    };
    let mut grouped = BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for y in 31673..=31680 {
        for x in 33288..=33299 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            let attributes = match (x, y) {
                (33292, 31677) => Some(sector::Attrs {
                    unique: Some(2285),
                    ..Default::default()
                }),
                (33292, 31678) => Some(sector::Attrs {
                    unique: Some(3024),
                    ..Default::default()
                }),
                (33294, 31678) => Some(sector::Attrs {
                    action: Some(12105),
                    ..Default::default()
                }),
                _ => None,
            };
            if let Some(attrs) = attributes {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs,
                });
            }
            grouped
                .entry((x / 32, y / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
    }
    let mut sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sx, sy), tiles)| bundle::Sector {
            floor: -15,
            sx,
            sy,
            tiles,
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 33293,
                y: 31677,
                floor: -15,
            },
        },
        crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
