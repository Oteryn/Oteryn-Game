//! Scoped Terrain uses the donor item only at its reviewed source cells.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde::Deserialize;
use serde_json::Value;

use crate::{Error, b3, bundle, project::LegacyPosition, sector::Attrs};

use super::read;

const CAPTURE: &str = "tools/world-bundle-compiler/scoped-spawn-ground.json";

#[derive(Deserialize)]
struct Capture {
    points: Vec<Point>,
}

#[derive(Deserialize)]
struct Point {
    cell: LegacyPosition,
    ground: u64,
}

pub(super) fn validate(
    root: &Path,
    index: &Value,
    regions: &[Vec<u8>],
    selected: Option<&[(u16, u16)]>,
) -> Result<(), Error> {
    let invalid = |detail: &str| Error::Format(format!("scoped spawn ground: {detail}"));
    let palette = index["palette"]
        .as_array()
        .ok_or_else(|| invalid("missing placements palette"))?;
    let path = root.join(CAPTURE);
    let has_markers = palette
        .iter()
        .any(|entry| entry.get("scoped_spawn_ground").is_some());
    if !has_markers && !path.try_exists().map_err(|err| invalid(&err.to_string()))? {
        return Ok(());
    }
    let capture: Capture =
        serde_json::from_slice(&read(&path)?).map_err(|err| invalid(&err.to_string()))?;
    let mut expected = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for Point { cell, ground } in capture.points {
        keys.insert(format!("oteryn:terrain.tibia.i{ground}"));
        if expected.insert(cell, ground).is_some() {
            return Err(invalid(&format!("duplicate captured cell {cell:?}")));
        }
    }
    if let Some(regions) = selected {
        expected.retain(|&(x, y, _), _| regions.contains(&(x / 256, y / 256)));
    }
    let mut slots = vec![None; palette.len()];
    for (entry, slot) in palette.iter().zip(&mut slots) {
        if let Some(marker) = entry.get("scoped_spawn_ground") {
            let item = entry["source_item_id"]
                .as_u64()
                .ok_or_else(|| invalid("missing source item ID"))?;
            let key = format!("oteryn:terrain.tibia.i{item}");
            if *marker != Value::Bool(true)
                || entry["provisional"] != Value::Bool(false)
                || entry["key"] != key
                || !keys.contains(&key)
            {
                return Err(invalid("invalid scoped palette entry"));
            }
            *slot = Some(item);
        } else if entry["key"].as_str().is_some_and(|key| keys.contains(key)) {
            return Err(invalid(
                "scoped Terrain key without a scoped palette marker",
            ));
        }
    }
    if has_markers {
        let mut budget = bundle::BUNDLE_BUDGET;
        for data in regions {
            let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
            for tile in region.sectors.iter().flat_map(|(_, _, tiles)| tiles) {
                let cell = (tile.x, tile.y, region.z);
                for entry in &tile.items {
                    if let Some(Some(item)) = slots.get(entry.palette as usize)
                        && (expected.remove(&cell) != Some(*item)
                            || entry.depth != 0
                            || entry.attrs != Attrs::default()
                            || tile.flags & 1 != 0
                            || tile.house != 0)
                    {
                        return Err(invalid(&format!("unapproved placement at {cell:?}")));
                    }
                }
            }
        }
    }
    if let Some((cell, _)) = expected.first_key_value() {
        return Err(invalid(&format!("missing placement at {cell:?}")));
    }
    Ok(())
}
