//! SP02: reset-epoch map projection, before collision, floor, spell and view owners are built.
//! Source: Crystal 201189f world_changes/{their_masters_voice,oriental_trader}.lua.
use super::*;
use oteryn_world_bundle::sector::{Attrs, Item, Tile};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LakeState {
    Clean,
    Dirty,
    Stinking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyMap {
    pub seed: [u8; 32],
    pub fungus: bool,
    pub grimvale_full_moon: bool,
    /// One of the three committed `oriental_trader/<town>` layer keys.
    pub trader: Option<&'static str>,
    /// The current saved World's lake projection; absent for legacy map fixtures.
    pub lake_state: Option<LakeState>,
}

impl DailyMap {
    fn pick(&self, x: u16, y: u16, purpose: &str, count: u64) -> u64 {
        ((u128::from(self.draw(x, y, purpose)) * u128::from(count)) >> 64) as u64
    }

    fn draw(&self, x: u16, y: u16, purpose: &str) -> u64 {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.world_change.physical.v1\0");
        hash.update(self.seed);
        hash.update(x.to_be_bytes());
        hash.update(y.to_be_bytes());
        hash.update(purpose.as_bytes());
        let bytes: [u8; 32] = hash.finalize().into();
        u64::from_be_bytes(bytes[..8].try_into().unwrap_or_default())
    }
}

pub(super) struct Projection {
    daily: DailyMap,
    manifest: Manifest,
}

impl Projection {
    pub(super) fn new(
        data: &[u8],
        daily: &DailyMap,
        item: &impl Fn(&str) -> Option<super::facts::ItemDefinition>,
    ) -> Result<Self, Error> {
        // CheckedBundle already verified these bytes and the manifest's bounds/digest.
        let length = data
            .get(8..12)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| refusal("manifest length"))? as usize;
        let bytes = data
            .get(bundle::HEADER..bundle::HEADER + length)
            .ok_or_else(|| refusal("manifest bounds"))?;
        let mut manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|_| refusal("manifest"))?;
        let mut needed = BTreeMap::new();
        if daily.fungus
            && let Some(ground) = manifest
                .palette
                .iter()
                .find(|p| palette_appearance(&p.key) == Some(12065))
                .and_then(|p| p.terrain)
        {
            for appearance in 12059..=12063 {
                needed.insert(appearance, Some(ground));
            }
        }
        if let Some(town) = daily.trader {
            for tile in source_layer(town)?.tiles {
                for (appearance, _, _) in tile.entries {
                    needed.entry(appearance).or_insert(None);
                }
            }
        }
        for (appearance, terrain) in needed {
            if manifest
                .palette
                .iter()
                .any(|p| palette_appearance(&p.key) == Some(appearance))
            {
                continue;
            }
            let key = format!("oteryn:item.tibia.i{appearance}");
            let definition =
                item(&key).ok_or_else(|| refusal(&format!("unserved appearance {appearance}")))?;
            manifest.palette.push(bundle::PaletteEntry {
                key,
                family: bundle::Family::Item,
                id: definition.reference.get() - 1,
                terrain,
            });
        }
        if let Some(state) = daily.lake_state {
            lake_palette(&mut manifest, state, item)?;
        }
        Ok(Self {
            daily: daily.clone(),
            manifest,
        })
    }
}

#[derive(Deserialize)]
struct Layers {
    layers: Vec<Layer>,
}
#[derive(Deserialize)]
struct Layer {
    key: String,
    tiles: Vec<LayerTile>,
}
#[derive(Deserialize)]
struct LayerTile {
    position: (u16, u16, i8),
    flags: Option<u32>,
    entries: Vec<(u16, Option<u16>, Option<u16>)>,
}

fn refusal(message: &str) -> Error {
    Error::Format(format!("daily world map: {message}"))
}
fn key(sector: &Sector) -> u32 {
    // Both the verified bundle and the committed layer have valid native floors/sectors.
    ((i32::from(sector.floor) + 15) as u32) << 22
        | u32::from(sector.sy) << 11
        | u32::from(sector.sx)
}
fn index(manifest: &Manifest, appearance: u16) -> Result<u32, Error> {
    manifest
        .palette
        .iter()
        .position(|p| palette_appearance(&p.key) == Some(appearance))
        .and_then(|i| u32::try_from(i).ok())
        .ok_or_else(|| refusal(&format!("unserved appearance {appearance}")))
}

fn source_layer(town: &str) -> Result<Layer, Error> {
    let document: Layers = serde_json::from_str(include_str!(
        "../../../../../content/encounters/world-changes/layers.json"
    ))
    .map_err(|_| refusal("invalid committed layers"))?;
    document
        .layers
        .into_iter()
        .find(|l| l.key == town)
        .ok_or_else(|| refusal("unknown trader town"))
}

fn layers(manifest: &Manifest, daily: &DailyMap) -> Result<BTreeMap<u32, Sector>, Error> {
    let Some(town) = daily.trader else {
        return Ok(BTreeMap::new());
    };
    let layer = source_layer(town)?;
    let mut out = BTreeMap::<u32, Sector>::new();
    for tile in layer.tiles {
        let (x, y, z) = tile.position;
        if x < manifest.world.min_x
            || x >= manifest.world.max_x
            || y < manifest.world.min_y
            || y >= manifest.world.max_y
            || !manifest.world.floors.contains(&-z)
        {
            return Err(refusal("trader layer outside world"));
        }
        let sector = Sector {
            floor: -z,
            sx: x / 32,
            sy: y / 32,
            tiles: Vec::new(),
        };
        let items = tile
            .entries
            .into_iter()
            .map(|(appearance, action, unique)| {
                Ok(Item {
                    palette: index(manifest, appearance)?,
                    depth: 0,
                    attrs: Attrs {
                        action,
                        unique,
                        ..Attrs::default()
                    },
                })
            })
            .collect::<Result<_, Error>>()?;
        out.entry(key(&sector)).or_insert(sector).tiles.push(Tile {
            x,
            y,
            flags: tile.flags.unwrap_or(0),
            house: 0,
            zones: Vec::new(),
            items,
        });
    }
    Ok(out)
}

fn fungus(manifest: &Manifest, sector: &mut Sector, daily: &DailyMap) -> Result<(), Error> {
    if !daily.fungus || sector.floor != -9 {
        return Ok(());
    }
    for tile in &mut sector.tiles {
        if !(33306..=33369).contains(&tile.x)
            || !(31847..=31919).contains(&tile.y)
            || daily.pick(tile.x, tile.y, "fungus-chance", 100) >= 30
        {
            continue;
        }
        for item in &mut tile.items {
            if item.depth == 0
                && manifest
                    .palette
                    .get(item.palette as usize)
                    .is_some_and(|p| palette_appearance(&p.key) == Some(12065))
            {
                let appearance = 12059 + (daily.pick(tile.x, tile.y, "fungus-kind", 5)) as u16;
                item.palette = index(manifest, appearance)?;
                break;
            }
        }
    }
    Ok(())
}

fn lake_appearance(state: LakeState, source: u16) -> Option<u16> {
    match (state, source) {
        (LakeState::Clean, 12558..=12563) => Some(4610),
        (LakeState::Dirty, 4608..=4613 | 12558..=12563) => Some(12561),
        (LakeState::Stinking, 4608..=4613 | 12558..=12563) => Some(12563),
        _ => None,
    }
}

fn lake_palette(
    manifest: &mut Manifest,
    state: LakeState,
    item: &impl Fn(&str) -> Option<super::facts::ItemDefinition>,
) -> Result<(), Error> {
    let needed: Vec<_> = manifest
        .palette
        .iter()
        .filter_map(|entry| {
            let appearance = lake_appearance(state, palette_appearance(&entry.key)?)?;
            Some((appearance, entry.terrain))
        })
        .collect();
    for (appearance, terrain) in needed {
        // A target appearance with different ground semantics cannot substitute
        // for this source palette entry. Preserve its exact terrain binding.
        if manifest.palette.iter().any(|entry| {
            palette_appearance(&entry.key) == Some(appearance) && entry.terrain == terrain
        }) {
            continue;
        }
        let key = format!("oteryn:item.tibia.i{appearance}");
        let definition =
            item(&key).ok_or_else(|| refusal(&format!("unserved lake appearance {appearance}")))?;
        manifest.palette.push(bundle::PaletteEntry {
            key,
            family: bundle::Family::Item,
            id: definition.reference.get() - 1,
            terrain,
        });
    }
    Ok(())
}

fn lake(manifest: &Manifest, sector: &mut Sector, state: LakeState) -> Result<(), Error> {
    if sector.floor != -7 {
        return Ok(());
    }
    // The committed native B3 region-z07-x127-y127 contains one connected
    // 499-tile component of source lake water12558..12563 in these bounds.
    // Region SHA25673a839c9ca9f5aa623bd1e6deade1babd23fd656dae8ae5ced8c6192184fc2e8.
    // Appearance policy is an owner-authorized fallback: blue4610 / dirty12561 /
    // heavily polluted12563. Ground semantics and every placement attribute stay intact.
    for tile in &mut sector.tiles {
        if tile.house != 0
            || !(32597..=32630).contains(&tile.x)
            || !(32642..=32671).contains(&tile.y)
        {
            continue;
        }
        for item in tile.items.iter_mut().filter(|item| item.depth == 0) {
            let source = &manifest.palette[item.palette as usize];
            let Some(appearance) = palette_appearance(&source.key)
                .and_then(|appearance| lake_appearance(state, appearance))
            else {
                continue;
            };
            item.palette = manifest
                .palette
                .iter()
                .position(|entry| {
                    palette_appearance(&entry.key) == Some(appearance)
                        && entry.terrain == source.terrain
                })
                .and_then(|index| u32::try_from(index).ok())
                .ok_or_else(|| refusal("lake palette loses source terrain"))?;
        }
    }
    Ok(())
}

/// Projects the same verified sectors for both the compact map and its fact owners. Nothing
/// is written to the bundle or its pins; rebuilding from the original bytes restores the day.
pub(super) fn visit(
    data: &[u8],
    limits: LoadLimits,
    projection: Option<&Projection>,
    mut visitor: impl FnMut(&Manifest, Sector) -> Result<(), Error>,
) -> Result<bundle::Visited, Error> {
    let Some(projection) = projection else {
        return bundle::visit_budgeted(data, limits.caps, limits.budget, visitor);
    };
    let daily = &projection.daily;
    let mut pending = None;
    let mut visited = bundle::visit_budgeted(data, limits.caps, limits.budget, |_, mut sector| {
        let manifest = &projection.manifest;
        let pending = match &mut pending {
            Some(pending) => pending,
            slot @ None => slot.insert(layers(manifest, daily)?),
        };
        while pending
            .first_key_value()
            .is_some_and(|(k, _)| *k < key(&sector))
        {
            if let Some((_, extra)) = pending.pop_first() {
                visitor(manifest, extra)?;
            }
        }
        if daily.trader == Some("oriental_trader/ankrahmun") && sector.floor == -6 {
            for tile in &mut sector.tiles {
                let remove = match (tile.x, tile.y) {
                    (33096, 32882) => &[4977][..],
                    (33096, 32883) => &[4977, 4920][..],
                    (33096, 32884..=32885) => &[4920][..],
                    _ => &[],
                };
                for appearance in remove {
                    if let Some(index) = tile.items.iter().position(|i| {
                        i.depth == 0
                            && manifest
                                .palette
                                .get(i.palette as usize)
                                .and_then(|p| palette_appearance(&p.key))
                                == Some(*appearance)
                    }) {
                        tile.items.remove(index);
                    }
                }
            }
        }
        if let Some(layer) = pending.remove(&key(&sector)) {
            // A source ship tile supplies the day's whole tile, including ground and flags.
            // Unchanged tiles in the sector retain their original placement attributes.
            let mut tiles: BTreeMap<_, _> =
                sector.tiles.into_iter().map(|t| ((t.y, t.x), t)).collect();
            for tile in layer.tiles {
                if tiles
                    .get(&(tile.y, tile.x))
                    .is_some_and(|base| base.house != 0)
                {
                    return Err(refusal("trader layer overlaps a house"));
                }
                tiles.insert((tile.y, tile.x), tile);
            }
            sector.tiles = tiles.into_values().collect();
        }
        fungus(manifest, &mut sector, daily)?;
        if let Some(state) = daily.lake_state {
            lake(manifest, &mut sector, state)?;
        }
        visitor(manifest, sector)
    })?;
    visited.manifest = projection.manifest.clone();
    if let Some(pending) = pending {
        for sector in pending.into_values() {
            visitor(&visited.manifest, sector)?;
        }
    }
    Ok(visited)
}

/// Crystal grimvale_respawn_event.lua:5-29. At most one eligible position per x-column.
/// `monster_functions.cpp:456` defaults setSpawnPosition() to 90 seconds at spawn rate 1.
pub(super) fn grimvale_spawns(
    world: &super::boot::BundleWorld,
    daily: &DailyMap,
    profile: impl Fn(&str) -> Option<(u64, bool)>,
) -> Option<Vec<super::boot::BundleSpawnFacts>> {
    use super::view::MapFacts;
    let creatures = if daily.grimvale_full_moon {
        &["oteryn:creature.wereboar", "oteryn:creature.werebadger"][..]
    } else {
        &[
            "oteryn:creature.bandit",
            "oteryn:creature.badger",
            "oteryn:creature.blue_butterfly",
            "oteryn:creature.yellow_butterfly",
        ][..]
    };
    let mut sources = Vec::new();
    for x in 33330..=33350 {
        for y in 31670..=31690 {
            if daily.pick(x, y, "grimvale-position", 1000) < 982 {
                continue;
            }
            let pos = super::overlay::TilePos { x, y, floor: -7 };
            if !world.enterable(pos) {
                continue;
            }
            let Some(tile) = world.base().tile(x, y, -7) else {
                continue;
            };
            let mut ordinal = 0;
            let mut water = false;
            for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
                if depth == 0 {
                    water |= world
                        .facts()
                        .base_entry(pos, ordinal, id)
                        .is_none_or(|f| f.appearance_id == 486);
                    ordinal += 1;
                }
            }
            if water {
                continue;
            }
            let creature =
                creatures[(daily.pick(x, y, "grimvale-kind", creatures.len() as u64)) as usize];
            let (health, blockable) = profile(creature)?;
            sources.push((
                vec![(
                    (i32::from(x), i32::from(y), 7),
                    90_000,
                    creature.to_owned(),
                    health,
                    blockable,
                )],
                super::boot::BUNDLE_OCCUPANCY_RETRY_MS,
            ));
            break;
        }
    }
    Some(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcontrib_lake_projection_preserves_custody_attributes_flags_and_terrain() -> Result<(), Error>
    {
        let terrain = bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(false),
            ground_speed: Some(0),
        };
        let mut manifest = Manifest {
            subareas: Default::default(),
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
            projection_class: "server".into(),
            compiler_version: "wcontrib-test".into(),
            build_class: bundle::BuildClass::Production,
            identity: bundle::Identity::default(),
            world: bundle::Extent {
                min_x: 32000,
                min_y: 32000,
                max_x: 33000,
                max_y: 33000,
                floors: vec![-7],
            },
            palette: vec![bundle::PaletteEntry {
                key: "oteryn:item.tibia.i4610".into(),
                family: bundle::Family::Item,
                id: 4610,
                terrain: Some(terrain),
            }],
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
            npcs: Default::default(),
        };
        for appearance in [12561, 12563] {
            manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:item.tibia.i{appearance}"),
                family: bundle::Family::Item,
                id: appearance,
                terrain: Some(terrain),
            });
        }
        let tile = Tile {
            x: 32613,
            y: 32655,
            flags: 17,
            house: 0,
            zones: Vec::new(),
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs {
                    action: Some(555),
                    unique: Some(888),
                    ..Attrs::default()
                },
            }],
        };
        let mut outside = tile.clone();
        outside.x = 32631;
        let mut house = tile.clone();
        house.house = 1;
        let mut nested = tile.clone();
        nested.items[0].depth = 1;
        let original = Sector {
            floor: -7,
            sx: 32613 / 32,
            sy: 32655 / 32,
            tiles: vec![tile, outside, house, nested],
        };
        let mut different_terrain = manifest.clone();
        different_terrain.palette[1].terrain = None;
        lake_palette(&mut different_terrain, LakeState::Dirty, &|key| {
            let appearance: u32 = key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
            Some(super::super::facts::ItemDefinition {
                reference: std::num::NonZeroU32::new(appearance + 1)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        })?;
        let mut cloned_terrain = original.clone();
        lake(&different_terrain, &mut cloned_terrain, LakeState::Dirty)?;
        assert_ne!(cloned_terrain.tiles[0].items[0].palette, 1);
        assert_eq!(
            different_terrain.palette[cloned_terrain.tiles[0].items[0].palette as usize].terrain,
            Some(terrain)
        );
        let mut dirty = original.clone();
        let mut other_floor = original.clone();
        other_floor.floor = -8;
        let original_other_floor = other_floor.clone();
        lake(&manifest, &mut other_floor, LakeState::Dirty)?;
        assert_eq!(other_floor, original_other_floor);
        lake(&manifest, &mut dirty, LakeState::Dirty)?;
        assert_eq!(dirty.tiles[0].items[0].palette, 1);
        assert_eq!(
            dirty.tiles[0].items[0].attrs,
            original.tiles[0].items[0].attrs
        );
        assert_eq!(dirty.tiles[0].flags, 17);
        assert_eq!(&dirty.tiles[1..], &original.tiles[1..]);
        assert_eq!(
            manifest.palette[dirty.tiles[0].items[0].palette as usize].terrain,
            Some(terrain)
        );
        let mut stinking = original.clone();
        lake(&manifest, &mut stinking, LakeState::Stinking)?;
        assert_eq!(stinking.tiles[0].items[0].palette, 2);
        lake(&manifest, &mut dirty, LakeState::Clean)?;
        assert_eq!(dirty, original);
        Ok(())
    }
}
