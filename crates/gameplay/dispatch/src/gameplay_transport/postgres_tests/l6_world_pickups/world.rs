//! Native B3 crop: preserve source tiles and lower their palette with the production registry.
use super::{TestResult, debug};
use crate::content::native_gameplay::NativeGameplayState;
use oteryn_world_bundle::{bundle, sector};
use oteryn_world_bundle_compiler::compile::{KeyResolver, Resolution};
use std::collections::BTreeMap;

pub(crate) struct ExpectedTile {
    pub position: (u16, u16),
    pub ids: Vec<u32>,
    pub counted_entry: Option<(usize, u32)>,
}

pub(crate) struct NativeCrop {
    pub bytes: &'static [u8],
    pub region: (u8, u16, u16),
    pub bounds: (u16, u16, u16, u16),
    pub start: (u16, u16),
    pub expected: Vec<ExpectedTile>,
}

pub(crate) fn small_ruby(
    native: &NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z15-x126-y125.b3"
            )),
            region: (15, 126, 125),
            bounds: (32436, 32173, 32439, 32176),
            start: (32436, 32174),
            expected: vec![ExpectedTile {
                position: (32437, 32174),
                ids: vec![103, 3030, 2123],
                counted_entry: Some((1, 1)),
            }],
        },
    )
}

#[derive(Clone, Copy)]
pub(crate) struct ExpectedTeleport {
    pub position: (u16, u16, u8),
    pub ordinal: usize,
    pub destination: (u16, u16, u8),
}

pub(crate) fn native_crop(
    native: &NativeGameplayState,
    spec: NativeCrop,
) -> TestResult<crate::map::boot::BundleWorld> {
    native_scene(native, vec![spec], &[])
}

pub(crate) fn native_scene(
    native: &NativeGameplayState,
    specs: Vec<NativeCrop>,
    teleports: &[ExpectedTeleport],
) -> TestResult<crate::map::boot::BundleWorld> {
    native_scene_with_appearances(native, specs, teleports, &[])
}

/// Keep source tiles unchanged while admitting source-script projection states
/// through the same production registry as the retained native base palette.
pub(crate) fn native_scene_with_appearances(
    native: &NativeGameplayState,
    specs: Vec<NativeCrop>,
    teleports: &[ExpectedTeleport],
    appearances: &[u16],
) -> TestResult<crate::map::boot::BundleWorld> {
    let first = specs.first().ok_or("native crop set")?;
    let start = (first.start.0, first.start.1, -i8::try_from(first.region.0)?);
    let min_x = specs.iter().map(|s| s.bounds.0).min().ok_or("minimumx")?;
    let min_y = specs.iter().map(|s| s.bounds.1).min().ok_or("minimumy")?;
    let max_x = specs.iter().map(|s| s.bounds.2).max().ok_or("maximumx")?;
    let max_y = specs.iter().map(|s| s.bounds.3).max().ok_or("maximumy")?;
    let floors = specs
        .iter()
        .map(|s| i8::try_from(s.region.0).map(|z| -z))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let project = oteryn_world_bundle_compiler::source::Project {
        index,
        regions: Vec::new(),
        families: Default::default(),
    };
    let (registry, source_palette) =
        oteryn_world_bundle_compiler::source::registry(&root, &project)?;
    let (fixture, mut pins) = crate::map::boot::tests::bundle();
    let mut manifest = bundle::read(&fixture)?.manifest;
    manifest.palette.clear();
    manifest.world = bundle::Extent {
        min_x,
        min_y,
        max_x,
        max_y,
        floors: floors.into_iter().collect(),
    };
    let mut remap = BTreeMap::new();
    let mut sectors: BTreeMap<(i8, u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    let item_index = native.item_index().ok_or("active Item index")?;
    let mut seen = std::collections::BTreeSet::new();
    let mut checked_teleports = std::collections::BTreeSet::new();
    for spec in specs {
        let (min_x, min_y, max_x, max_y) = spec.bounds;
        let floor = -i8::try_from(spec.region.0)?;
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            spec.bytes,
            bundle::TILE_LIMITS,
            &mut sector::Budget {
                tiles: 65536,
                entries: 1_000_000,
            },
        )?;
        assert_eq!((region.z, region.rx, region.ry), spec.region);
        let mut checked = std::collections::BTreeSet::new();
        for (_, _, tiles) in region.sectors {
            for mut tile in tiles {
                if !(min_x..max_x).contains(&tile.x) || !(min_y..max_y).contains(&tile.y) {
                    continue;
                }
                assert_eq!(tile.house, 0, "this source crop is outside Houses");
                for expected in &spec.expected {
                    if (tile.x, tile.y) != expected.position {
                        continue;
                    }
                    let ids = tile
                        .items
                        .iter()
                        .map(|item| {
                            project.index["palette"][item.palette as usize]["source_item_id"]
                                .as_u64()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        ids,
                        expected
                            .ids
                            .iter()
                            .map(|id| Some(u64::from(*id)))
                            .collect::<Vec<_>>()
                    );
                    assert!(tile.items.iter().all(|item| item.depth == 0));
                    if let Some((ordinal, count)) = expected.counted_entry {
                        assert_eq!(tile.items[ordinal].attrs.count.map(u32::from), Some(count));
                    }
                    checked.insert(expected.position);
                }
                assert!(
                    seen.insert((floor, tile.x, tile.y)),
                    "native crops do not overlap"
                );
                for (ordinal, item) in tile.items.iter_mut().enumerate() {
                    if let Some(destination) = item.attrs.teleport {
                        assert!(
                            teleports.iter().any(|expected| expected.position
                                == (tile.x, tile.y, spec.region.0)
                                && expected.ordinal == ordinal
                                && expected.destination == destination),
                            "every source teleport is explicitly witnessed before coordinate lowering"
                        );
                        checked_teleports.insert((tile.x, tile.y, spec.region.0, ordinal));
                        item.attrs.teleport = Some((
                            destination.0,
                            destination.1,
                            (-i8::try_from(destination.2)?) as u8,
                        ));
                    }
                    let original = item.palette;
                    let palette = match remap.get(&original) {
                        Some(palette) => *palette,
                        None => {
                            let key = source_palette
                                .get(original as usize)
                                .ok_or("source palette")?;
                            let Resolution::Resolved(family, id) = registry.resolve(key) else {
                                return Err(format!("unresolved native crop key:{key}").into());
                            };
                            if family == bundle::Family::Item {
                                assert_eq!(
                                    item_index
                                        .definition_ref("Item", key, "definition-r1")
                                        .ok_or("active native crop Item")?
                                        .get(),
                                    id + 1
                                );
                            }
                            let palette = u32::try_from(manifest.palette.len())?;
                            manifest.palette.push(bundle::PaletteEntry {
                                key: key.clone(),
                                family,
                                id,
                                terrain: registry.terrain(key)?,
                            });
                            remap.insert(original, palette);
                            palette
                        }
                    };
                    item.palette = palette;
                }
                sectors
                    .entry((floor, tile.x / 32, tile.y / 32))
                    .or_default()
                    .push(tile);
            }
        }
        assert_eq!(
            checked.len(),
            spec.expected.len(),
            "every expected source tile is present"
        );
    }
    assert_eq!(
        checked_teleports.len(),
        teleports.len(),
        "every declared source teleport was retained"
    );
    for appearance in appearances {
        let key = format!("oteryn:item.tibia.i{appearance}");
        if manifest.palette.iter().any(|entry| entry.key == key) {
            continue;
        }
        let Resolution::Resolved(family, id) = registry.resolve(&key) else {
            return Err(format!("unresolved source projection:{key}").into());
        };
        if family == bundle::Family::Item {
            assert_eq!(
                item_index
                    .definition_ref("Item", &key, "definition-r1")
                    .ok_or("native projection Item")?
                    .get(),
                id + 1
            );
        }
        manifest.palette.push(bundle::PaletteEntry {
            terrain: registry.terrain(&key)?,
            key,
            family,
            id,
        });
    }
    let mut sectors = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let definitions = |key: &str| {
        let profile = native
            .item_policy(key, "definition-r1")
            .map(|policy| policy.record());
        Some(crate::map::facts::ItemDefinition {
            reference: item_index.definition_ref("Item", key, "definition-r1")?,
            solid: profile.and_then(|profile| profile.attributes.blocks_movement),
            blocks_projectile: profile
                .and_then(|profile| profile.attributes.blocks_projectile)
                .unwrap_or(true),
            pickupable: profile.is_some_and(|profile| {
                matches!(&profile.semantics.physical,
                crate::content::ReferenceItemField::Known(physical)
                if physical.pickupable == crate::content::ReferenceItemField::Known(true))
            }),
            door: profile.is_some_and(|profile| {
                matches!(&profile.semantics.classification,
                    crate::content::ReferenceItemField::Known(classification)
                    if classification.item_type == crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::Door))
            }),
        })
    };
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: start.0,
                y: start.1,
                floor: start.2,
            },
        },
        crate::foundation::WorldId::decode(&super::super::fixture::id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&super::super::fixture::id(43)).map_err(debug)?,
        definitions,
    )
    .map_err(debug)
}
