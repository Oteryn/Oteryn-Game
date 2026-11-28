//! Load an unmodified crop of the committed B3 source through the native resolver.

use super::{
    TestResult,
    fixture::{debug, id},
};

use oteryn_world_bundle::bundle;

use oteryn_world_bundle_compiler::{
    b3,
    compile::{KeyResolver, Resolution},
    source,
};

pub(super) fn world(
    minimum: crate::map::overlay::TilePos,
    maximum: crate::map::overlay::TilePos,
    start: crate::map::overlay::TilePos,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let project = source::Project {
        index,
        regions: Vec::new(),
        families: Default::default(),
    };
    let (registry, palette) = source::registry(&root, &project)?;
    if minimum.floor != maximum.floor
        || minimum.floor != start.floor
        || !(-15..=0).contains(&minimum.floor)
        || minimum.x > maximum.x
        || minimum.y > maximum.y
        || minimum.x / 256 != maximum.x / 256
        || minimum.y / 256 != maximum.y / 256
    {
        return Err("invalid source crop".into());
    }
    let source_floor = -i16::from(minimum.floor);
    let path = format!(
        "content/world/placements/region-z{source_floor:02}-x{}-y{}.b3",
        minimum.x / 256,
        minimum.y / 256
    );
    let raw = std::fs::read(root.join(path))?;
    let mut budget = bundle::BUNDLE_BUDGET;
    let region = b3::decode_region(&raw, bundle::TILE_LIMITS, &mut budget)?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    let mut remap = std::collections::BTreeMap::new();
    let mut sectors = Vec::new();
    for (sx, sy, tiles) in region.sectors {
        let mut kept = Vec::new();
        for mut tile in tiles {
            if !(minimum.x..=maximum.x).contains(&tile.x)
                || !(minimum.y..=maximum.y).contains(&tile.y)
            {
                continue;
            }
            for item in &mut tile.items {
                if item.attrs.teleport.is_some() {
                    return Err("source crop requires teleport-family compilation".into());
                }
                let mapped = if let Some(mapped) = remap.get(&item.palette) {
                    *mapped
                } else {
                    let key = palette
                        .get(usize::try_from(item.palette)?)
                        .ok_or("source palette")?;
                    let Resolution::Resolved(family, id) = registry.resolve(key) else {
                        return Err(format!("unresolved source item {key}").into());
                    };
                    let mapped = u32::try_from(data.manifest.palette.len())?;
                    data.manifest.palette.push(bundle::PaletteEntry {
                        key: key.clone(),
                        family,
                        id,
                        terrain: registry.terrain(key)?,
                    });
                    remap.insert(item.palette, mapped);
                    mapped
                };
                item.palette = mapped;
            }
            kept.push(tile);
        }
        if !kept.is_empty() {
            sectors.push(bundle::Sector {
                floor: minimum.floor,
                sx,
                sy,
                tiles: kept,
            });
        }
    }
    if sectors.is_empty() {
        return Err("source crop contains no tiles".into());
    }
    data.manifest.world = bundle::Extent {
        min_x: minimum.x,
        min_y: minimum.y,
        max_x: maximum.x.checked_add(1).ok_or("crop x")?,
        max_y: maximum.y.checked_add(1).ok_or("crop y")?,
        floors: vec![minimum.floor],
    };
    let mut missing = Vec::new();
    for entry in &data.manifest.palette {
        if entry.family == bundle::Family::Item {
            let reference = native
                .item_index()
                .and_then(|index| index.definition_ref("Item", &entry.key, "definition-r1"));
            if reference.map(|r| r.get()) != entry.id.checked_add(1) {
                missing.push(format!(
                    "{}: palette={}, native={reference:?}",
                    entry.key, entry.id
                ));
            }
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "source crop Item admission mismatch: {}",
            missing.join("; ")
        )
        .into());
    }
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

/// Compile the actual source region when the route spans floors or includes
/// teleport attributes. The repository compiler owns family parity, floor signs,
/// source subareas and canonical definitions; no legacy attribute is hand-converted.
pub(super) fn compiled_region(
    minimum: crate::map::overlay::TilePos,
    maximum: crate::map::overlay::TilePos,
    start: crate::map::overlay::TilePos,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    if minimum.x / 256 != maximum.x / 256 || minimum.y / 256 != maximum.y / 256 {
        return Err("invalid source region route bounds".into());
    }
    compiled_regions(minimum, maximum, start, native)
}

pub(super) fn compiled_regions(
    minimum: crate::map::overlay::TilePos,
    maximum: crate::map::overlay::TilePos,
    start: crate::map::overlay::TilePos,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    compile_regions(minimum, maximum, start, native, |diagnostic| {
        (minimum.floor..=maximum.floor).contains(&diagnostic.floor)
            && (minimum.x..=maximum.x).contains(&diagnostic.x)
            && (minimum.y..=maximum.y).contains(&diagnostic.y)
    })
}

/// A direct STEP passage uses its approach, trigger and landing on their source floors.
pub(super) fn compiled_step_passage(
    cells: [crate::map::overlay::TilePos; 3],
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::overlay::TilePos;

    let [start, trigger, landing] = cells;
    let minimum = TilePos {
        x: start.x.min(trigger.x).min(landing.x),
        y: start.y.min(trigger.y).min(landing.y),
        floor: start.floor.min(trigger.floor).min(landing.floor),
    };
    let maximum = TilePos {
        x: start.x.max(trigger.x).max(landing.x),
        y: start.y.max(trigger.y).max(landing.y),
        floor: start.floor.max(trigger.floor).max(landing.floor),
    };
    compile_regions(minimum, maximum, start, native, |diagnostic| {
        cells.iter().any(|cell| {
            (cell.x, cell.y, cell.floor) == (diagnostic.x, diagnostic.y, diagnostic.floor)
        })
    })
}

fn compile_regions(
    minimum: crate::map::overlay::TilePos,
    maximum: crate::map::overlay::TilePos,
    start: crate::map::overlay::TilePos,
    native: &crate::content::native_gameplay::NativeGameplayState,
    is_route_geometry: impl Fn(&oteryn_world_bundle_compiler::compile::Diagnostic) -> bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{compile, project};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if minimum.x > maximum.x
        || minimum.y > maximum.y
        || minimum.floor > maximum.floor
        || !(minimum.floor..=maximum.floor).contains(&start.floor)
    {
        return Err("invalid source region route bounds".into());
    }
    let regions: Vec<_> = (minimum.x / 256..=maximum.x / 256)
        .flat_map(|x| (minimum.y / 256..=maximum.y / 256).map(move |y| (x, y)))
        .collect();
    let project = source::project_regions(&root, &regions)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    let unresolved = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| is_route_geometry(diagnostic));
    assert!(unresolved.is_none(), "unresolved source quest geometry: {unresolved:?}");
    let pins = crate::map::BundlePins {
        digest: compiled.digest,
        ..pins
    };
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}
