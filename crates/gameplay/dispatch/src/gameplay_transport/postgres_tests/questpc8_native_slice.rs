//! Bounded real B3 map fixtures; use the compiler's registry and server boot semantics.
use super::TestResult;
use super::fixture::{debug, id};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::map::overlay::TilePos;
use oteryn_world_bundle::{bundle, sector};
use oteryn_world_bundle_compiler::{
    compile::{KeyResolver, Resolution},
    source,
};
use std::collections::BTreeMap;

pub(super) fn world(
    native: &NativeGameplayState,
    patches: &[(u16, u16, u16, u16, i8)],
    prototypes: &[u16],
    start: TilePos,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let directory = root.join("content/world/placements");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json"))?)?;
    let project = source::Project {
        index,
        regions: Vec::new(),
        families: Default::default(),
    };
    let (registry, keys) = source::registry(&root, &project)?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    if patches.is_empty() || patches.len() > 16 {
        return Err("native patch bounds".into());
    }
    let mut palette = BTreeMap::<String, u32>::new();
    let mut add = |key: &str| -> TestResult<u32> {
        if let Some(id) = palette.get(key) {
            return Ok(*id);
        }
        let Resolution::Resolved(family, id) = registry.resolve(key) else {
            return Err(format!("unresolved native fixture key {key}").into());
        };
        let ordinal = u32::try_from(data.manifest.palette.len())?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key: key.into(),
            family,
            id,
            terrain: registry.terrain(key)?,
        });
        palette.insert(key.into(), ordinal);
        Ok(ordinal)
    };
    let mut sectors = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    let mut withheld = 0usize;
    for &(px0, py0, px1, py1, floor) in patches {
        for ry in py0 / 256..=py1 / 256 {
            for rx in px0 / 256..=px1 / 256 {
                let mut budget = sector::Budget {
                    tiles: 65536,
                    entries: 1_000_000,
                };
                let region = oteryn_world_bundle_compiler::b3::decode_region(
                    &std::fs::read(
                        directory.join(format!("region-z{:02}-x{rx:03}-y{ry:03}.b3", -floor)),
                    )?,
                    bundle::TILE_LIMITS,
                    &mut budget,
                )?;
                for (_, _, tiles) in region.sectors {
                    for mut tile in tiles.into_iter().filter(|tile| {
                        (px0..=px1).contains(&tile.x) && (py0..=py1).contains(&tile.y)
                    }) {
                        // A provisional source entry is not an admitted collision fact.
                        // Withhold its entire tile as ABSENT instead of making it passable.
                        if tile.items.iter().any(|item| {
                            keys.get(item.palette as usize)
                                .is_some_and(|key| registry.resolve(key) == Resolution::Provisional)
                        }) {
                            withheld += 1;
                            continue;
                        }
                        for item in &mut tile.items {
                            item.palette = add(keys
                                .get(usize::try_from(item.palette)?)
                                .ok_or("source palette")?)?;
                        }
                        sectors
                            .entry((floor, tile.y / 32, tile.x / 32))
                            .or_default()
                            .push(tile);
                    }
                }
            }
        }
    }
    eprintln!("NATIVE_SOURCE_FIXTURE withheld_provisional_tiles={withheld} policy=ABSENT");
    for appearance in prototypes {
        add(&format!("oteryn:item.tibia.i{appearance}"))?;
    }
    let sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sy,
                sx,
                tiles,
            }
        })
        .collect();
    // Preserve the original coordinate frame. Source teleport destinations may
    // be outside the selected patches; their unloaded cells remain ABSENT.
    data.manifest.world = source::world_extent(&root)?;
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    // Same fail-closed item definition projection as node::serve::boot_bundle_items.
    let item = |key: &str| {
        let index = native.item_index()?;
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = native
            .item_policy(key, revision)
            .map(|policy| policy.record());
        Some(crate::map::facts::ItemDefinition {
            reference,
            solid:profile.and_then(|p|p.attributes.blocks_movement),
            blocks_projectile:profile.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
            pickupable:profile.is_some_and(|p|matches!(&p.semantics.physical,ReferenceItemField::Known(f) if f.pickupable==ReferenceItemField::Known(true))),
            door:profile.is_some_and(|p|matches!(&p.semantics.classification,ReferenceItemField::Known(f) if f.item_type==ReferenceItemField::Known(ReferenceItemType::Door))),
        })
    };
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        item,
    )
    .map_err(debug)
}
