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
    let mut palette = bundle.manifest.palette.clone();
    let mut occupied = BTreeSet::new();
    let mut replacements = BTreeMap::new();
    for key in selected {
        let layer = bundle
            .layers
            .iter()
            .find(|layer| layer.key == *key)
            .ok_or_else(|| Error::Key(format!("layer not compiled: {key}")))?;
        let offset =
            u32::try_from(palette.len()).map_err(|_| Error::Limit("layer palette".into()))?;
        palette.extend_from_slice(&layer.palette);
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
    for s in &bundle.sectors {
        for tile in &s.tiles {
            sectors.insert((s.floor, s.sy, s.sx, tile.y, tile.x), tile.clone());
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
    bundle.manifest.palette = palette;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{BuildClass, Extent, FORMAT, Family, Identity, Manifest, VERSION};
    #[test]
    fn failed_composition_preserves_palette_and_sectors() {
        let manifest = Manifest {
            subareas: Default::default(),
            format: FORMAT.into(),
            min_reader_version: VERSION,
            projection_class: "server".into(),
            compiler_version: "test".into(),
            build_class: BuildClass::Production,
            identity: Identity::default(),
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 1,
                max_y: 1,
                floors: vec![-7],
            },
            palette: vec![],
            draft_areas: vec![],
            skipped_provisional_keys: vec![],
            dropped_teleports: vec![],
            spawns: Default::default(),
            npcs: Default::default(),
        };

        let tile = sector::Tile {
            x: 0,
            y: 0,
            flags: 0,
            house: 1,
            zones: vec![],
            items: vec![],
        };
        let sector = Sector {
            floor: 0,
            sx: 0,
            sy: 0,
            tiles: vec![tile],
        };
        let palette = vec![PaletteEntry {
            key: "item/test".into(),
            family: Family::Item,
            id: 0,
            terrain: None,
        }];
        let mut layer_sector = sector.clone();
        layer_sector.tiles[0].house = 0;
        let layer = Layer {
            key: "event".into(),
            palette,
            sectors: vec![layer_sector],
        };
        for selection in [
            vec!["event"],
            vec!["event", "missing"],
            vec!["event", "event"],
        ] {
            let mut bundle = crate::bundle::Bundle {
                manifest: manifest.clone(),
                sectors: vec![sector.clone()],
                layers: vec![layer.clone()],
                spawns: Default::default(),
                npcs: Default::default(),
                digest: [0; 32],
            };
            assert!(compose(&mut bundle, &selection).is_err());
            assert_eq!(bundle.manifest, manifest);
            assert_eq!(bundle.sectors, vec![sector.clone()]);
        }
    }
}
