//! W1 optional layer admission through the existing World compiler.
use crate::{
    Error, b3, bundle,
    compile::{Input, KeyResolver, Resolution},
};
use oteryn_gameplay_world_change::variants::{self, Tile};
use oteryn_world_bundle::world_changes::{CAPABILITY, Package};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Layers {
    layers: Vec<Layer>,
}
#[derive(Deserialize)]
struct Layer {
    key: String,
    tiles: Vec<Tile>,
}

pub fn admit(input: &Input<'_>, resolver: &dyn KeyResolver) -> Result<Option<Package>, Error> {
    if !input
        .identity
        .required_capabilities
        .iter()
        .any(|key| key == CAPABILITY)
    {
        return Ok(None);
    }
    let definitions = variants::definitions().map_err(|e| Error::Format(e.into()))?;
    let layers: Layers =
        serde_json::from_str(variants::LAYERS).map_err(|e| Error::Format(e.to_string()))?;
    let layers: BTreeMap<_, _> = layers
        .layers
        .into_iter()
        .map(|l| (l.key, l.tiles))
        .collect();
    let mut positions = BTreeSet::new();
    let mut check = |tile: &Tile| -> Result<(), Error> {
        let (x, y, z) = tile.position;
        if !(0..=15).contains(&z)
            || x < input.world.min_x
            || x >= input.world.max_x
            || y < input.world.min_y
            || y >= input.world.max_y
            || !input.world.floors.contains(&-z)
        {
            return Err(Error::Bounds(format!("world change layer at {x},{y},{z}")));
        }
        positions.insert((x, y, z as u8));
        for (appearance, _, _) in &tile.entries {
            let key = format!("oteryn:item.tibia.i{appearance}");
            if !matches!(
                resolver.resolve(&key),
                Resolution::Resolved(bundle::Family::Item, _)
            ) {
                return Err(Error::Key(format!("world change item {key}")));
            }
        }
        Ok(())
    };
    for definition in definitions {
        for station in &definition.stations {
            check(&Tile {
                position: station.position,
                flags: None,
                entries: vec![(station.appearance, None, None)],
            })?;
            if !matches!(
                resolver.resolve(&station.material),
                Resolution::Resolved(bundle::Family::Item, _)
            ) {
                return Err(Error::Key(format!(
                    "world change material {}",
                    station.material
                )));
            }
        }
        for tile in &definition.inactive_tiles {
            check(tile)?;
        }
        for variant in &definition.variants {
            for portal in &variant.portals {
                check(&Tile {
                    position: portal.position,
                    flags: None,
                    entries: vec![(portal.appearance, portal.action, portal.unique)],
                })?;
            }
            for key in &variant.layers {
                for tile in layers
                    .get(key)
                    .ok_or_else(|| Error::Family(format!("missing world change layer {key}")))?
                {
                    check(tile)?;
                }
            }
            for tile in &variant.tiles {
                check(tile)?;
            }
            for tag in &variant.tags {
                check(&Tile {
                    position: tag.position,
                    flags: None,
                    entries: vec![(tag.appearance, tag.action, tag.unique)],
                })?;
            }
        }
    }
    let mut budget = bundle::BUNDLE_BUDGET;
    for data in input.regions {
        let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
        for tile in region.sectors.iter().flat_map(|(_, _, tiles)| tiles) {
            if tile.house != 0 && positions.contains(&(tile.x, tile.y, region.z)) {
                return Err(Error::Family(format!(
                    "world change layer touches house {} at {},{},{}",
                    tile.house, tile.x, tile.y, region.z
                )));
            }
        }
    }
    Ok(Some(Package::new(
        variants::RECIPES.into(),
        variants::LAYERS.into(),
    )))
}
