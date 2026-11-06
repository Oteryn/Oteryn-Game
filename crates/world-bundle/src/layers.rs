//! Optional, palette-resolved epoch layers (EVENTS-WORLD-1 W4).
use crate::{
    Error,
    bundle::{Manifest, PaletteEntry, Sector},
    sector,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub key: String,
    pub palette: Vec<PaletteEntry>,
    pub sectors: Vec<Sector>,
}

pub fn validate(manifest: &Manifest, layers: &[Layer]) -> Result<(), Error> {
    if layers.len() > 16 || !layers.windows(2).all(|p| p[0].key < p[1].key) {
        return Err(Error::Format("layer count/order".into()));
    }
    for layer in layers {
        if layer.key.is_empty()
            || layer.sectors.len() > 64
            || layer.sectors.is_empty()
            || !layer
                .sectors
                .windows(2)
                .all(|p| crate::bundle::order(&p[0]) < crate::bundle::order(&p[1]))
        {
            return Err(Error::Format("layer key/sector bounds/order".into()));
        }
        let mut local = manifest.clone();
        local.palette.clone_from(&layer.palette);
        crate::bundle::validate_manifest(&local)?;
        let mut budget = sector::Budget {
            tiles: 4096,
            entries: 65536,
        };
        for s in &layer.sectors {
            if s.tiles.iter().any(|tile| tile.house != 0) {
                return Err(Error::Family("layer touches a house".into()));
            }
            crate::bundle::validate_sector(&local, s)?;
            let raw = sector::encode(&s.tiles)?;
            let decoded =
                sector::decode(&raw, (s.sx, s.sy), crate::bundle::TILE_LIMITS, &mut budget)?;
            if decoded != s.tiles {
                return Err(Error::Format("layer sector coordinates".into()));
            }
        }
    }
    Ok(())
}

/// Compose only layers admitted by this bundle; the caller supplies the epoch selection.
/// No layer precedence is invented: overlapping selections are refused.
pub fn compose(bundle: &mut crate::bundle::Bundle, selected: &[&str]) -> Result<(), Error> {
    let mut occupied = BTreeSet::new();
    let mut replacements = BTreeMap::new();
    for key in selected {
        let layer = bundle
            .layers
            .iter()
            .find(|layer| layer.key == *key)
            .ok_or_else(|| Error::Key(format!("layer not compiled: {key}")))?;
        let offset = u32::try_from(bundle.manifest.palette.len())
            .map_err(|_| Error::Limit("layer palette".into()))?;
        bundle.manifest.palette.extend_from_slice(&layer.palette);
        for sector in &layer.sectors {
            for tile in &sector.tiles {
                if !occupied.insert((sector.floor, tile.y, tile.x)) {
                    return Err(Error::Family("selected layers overlap".into()));
                }
                let mut tile = tile.clone();
                for item in &mut tile.items {
                    item.palette = item
                        .palette
                        .checked_add(offset)
                        .ok_or_else(|| Error::Limit("layer palette".into()))?;
                }
                replacements.insert((sector.floor, sector.sy, sector.sx, tile.y, tile.x), tile);
            }
        }
    }
    let mut sectors = BTreeMap::new();
    for s in std::mem::take(&mut bundle.sectors) {
        for tile in s.tiles {
            sectors.insert((s.floor, s.sy, s.sx, tile.y, tile.x), tile);
        }
    }
    for (at, tile) in &replacements {
        if sectors.get(at).is_some_and(|base| base.house != 0) {
            return Err(Error::Family("layer intersects base house".into()));
        }
        sectors.insert(*at, tile.clone());
    }
    let mut grouped: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for ((floor, sy, sx, _, _), tile) in sectors {
        grouped.entry((floor, sy, sx)).or_default().push(tile);
    }
    let tiles: usize = grouped.values().map(Vec::len).sum();
    let entries: usize = grouped
        .values()
        .flatten()
        .map(|tile| tile.items.len())
        .sum();
    if tiles > crate::bundle::BUNDLE_BUDGET.tiles || entries > crate::bundle::BUNDLE_BUDGET.entries
    {
        return Err(Error::Limit("composed layer totals".into()));
    }
    bundle.sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sy,
            sx,
            tiles,
        })
        .collect();
    Ok(())
}
