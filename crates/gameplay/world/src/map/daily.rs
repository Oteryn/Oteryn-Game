//! SP02: reset-epoch map projection, before collision, floor, spell and view owners are built.
//! Source: Crystal 201189f world_changes/{their_masters_voice,oriental_trader}.lua.
use super::*;
use oteryn_world_bundle::sector::{Attrs, Item, Tile};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyMap {
    pub seed: [u8; 32],
    pub fungus: bool,
    pub grimvale_full_moon: bool,
    /// One of the three committed `oriental_trader/<town>` layer keys.
    pub trader: Option<&'static str>,
    pub variants: oteryn_gameplay_world_change::variants::Selected,
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

pub(super) fn requires_decision(data: &[u8]) -> Result<bool, Error> {
    #[derive(Deserialize)]
    struct Presence {
        world_changes: Option<serde::de::IgnoredAny>,
    }
    let length = data
        .get(8..12)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| refusal("manifest length"))? as usize;
    let bytes = data
        .get(bundle::HEADER..bundle::HEADER + length)
        .ok_or_else(|| refusal("manifest bounds"))?;
    let presence: Presence = serde_json::from_slice(bytes).map_err(|_| refusal("manifest"))?;
    Ok(presence.world_changes.is_some())
}

impl Projection {
    pub(super) fn barriers(
        &self,
    ) -> impl Iterator<Item = oteryn_gameplay_world_change::variants::Position> + '_ {
        self.manifest.world_changes.iter().flat_map(move |_| {
            self.daily
                .variants
                .active()
                .flat_map(|variant| variant.barriers.iter().copied())
        })
    }
    pub(super) fn world_changes_admitted(&self) -> bool {
        self.manifest.world_changes.is_some()
    }
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
        if let Some(package) = &manifest.world_changes {
            package.validate()?;
            if package.digest != oteryn_gameplay_world_change::variants::recipe_digest() {
                return Err(refusal(
                    "World change package differs from active server content",
                ));
            }
        } else if daily.variants.active().next().is_some() {
            return Err(refusal(
                "active World change has no admitted bundle package",
            ));
        }
        let mut needed = BTreeMap::new();
        for portal in manifest
            .world_changes
            .iter()
            .flat_map(|_| daily.variants.portals())
        {
            needed.entry(portal.appearance).or_insert(None);
        }
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
        for town in daily.trader.into_iter().chain(
            daily
                .variants
                .active()
                .flat_map(|v| v.layers.iter().map(String::as_str)),
        ) {
            for tile in source_layer(&manifest, town)?.tiles {
                for (appearance, _, _) in tile.entries {
                    needed.entry(appearance).or_insert(None);
                }
            }
        }
        for tile in daily.variants.active().flat_map(|v| &v.tiles) {
            for (appearance, _, _) in &tile.entries {
                needed.entry(*appearance).or_insert(None);
            }
        }
        for tile in manifest
            .world_changes
            .iter()
            .flat_map(|_| {
                oteryn_gameplay_world_change::variants::definitions()
                    .into_iter()
                    .flatten()
            })
            .flat_map(|d| &d.inactive_tiles)
        {
            for (appearance, _, _) in &tile.entries {
                needed.entry(*appearance).or_insert(None);
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

fn source_layer(manifest: &Manifest, town: &str) -> Result<Layer, Error> {
    let document: Layers = serde_json::from_str(
        manifest
            .world_changes
            .as_ref()
            .map_or(oteryn_gameplay_world_change::variants::LAYERS, |p| {
                p.layers.as_str()
            }),
    )
    .map_err(|_| refusal("invalid committed layers"))?;
    document
        .layers
        .into_iter()
        .find(|l| l.key == town)
        .ok_or_else(|| refusal("unknown trader town"))
}

fn layers(manifest: &Manifest, daily: &DailyMap) -> Result<BTreeMap<u32, Sector>, Error> {
    let mut out = BTreeMap::<u32, Sector>::new();
    let mut tiles = Vec::new();
    tiles.extend(
        manifest
            .world_changes
            .iter()
            .flat_map(|_| {
                oteryn_gameplay_world_change::variants::definitions()
                    .into_iter()
                    .flatten()
            })
            .flat_map(|d| &d.inactive_tiles)
            .map(|t| LayerTile {
                position: t.position,
                flags: t.flags,
                entries: t.entries.clone(),
            }),
    );
    for layer in daily.trader.into_iter().chain(
        daily
            .variants
            .active()
            .flat_map(|v| v.layers.iter().map(String::as_str)),
    ) {
        tiles.extend(source_layer(manifest, layer)?.tiles);
    }
    tiles.extend(
        daily
            .variants
            .active()
            .flat_map(|v| &v.tiles)
            .map(|t| LayerTile {
                position: t.position,
                flags: t.flags,
                entries: t.entries.clone(),
            }),
    );
    for tile in tiles {
        let (x, y, z) = tile.position;
        if x < manifest.world.min_x
            || x >= manifest.world.max_x
            || y < manifest.world.min_y
            || y >= manifest.world.max_y
            || !manifest.world.floors.contains(&-z)
        {
            return Err(refusal("variant layer outside world"));
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
        let output = &mut out.entry(key(&sector)).or_insert(sector).tiles;
        if let Some(previous) = output.iter().position(|t| t.x == x && t.y == y) {
            output.remove(previous);
        }
        output.push(Tile {
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

fn project_sector(
    manifest: &Manifest,
    daily: &DailyMap,
    mut sector: Sector,
) -> Result<Sector, Error> {
    sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
    fungus(manifest, &mut sector, daily)?;
    for portal in manifest
        .world_changes
        .iter()
        .flat_map(|_| daily.variants.portals())
    {
        let (x, y, z) = portal.position;
        if sector.floor != -z {
            continue;
        }
        if let Some(tile) = sector.tiles.iter_mut().find(|t| t.x == x && t.y == y) {
            if tile.house != 0 {
                return Err(refusal("variant portal overlaps a house"));
            }
            if let Some(item) = tile.items.iter_mut().find(|i| {
                i.depth == 0
                    && manifest
                        .palette
                        .get(i.palette as usize)
                        .and_then(|p| palette_appearance(&p.key))
                        == Some(portal.appearance)
            }) {
                item.attrs.action = portal.action;
                item.attrs.unique = portal.unique;
            } else {
                tile.items.push(Item {
                    palette: index(manifest, portal.appearance)?,
                    depth: 0,
                    attrs: Attrs {
                        action: portal.action,
                        unique: portal.unique,
                        ..Attrs::default()
                    },
                });
            }
        }
    }
    // Managed passages are never admitted by raw base teleport attributes.
    // Their current World/Character/session consumer is the sole entry owner.
    for portal in manifest
        .world_changes
        .iter()
        .flat_map(|_| {
            oteryn_gameplay_world_change::variants::definitions()
                .into_iter()
                .flatten()
        })
        .flat_map(|d| &d.variants)
        .flat_map(|v| &v.portals)
    {
        let (x, y, z) = portal.position;
        if sector.floor == -z
            && let Some(tile) = sector.tiles.iter_mut().find(|t| t.x == x && t.y == y)
        {
            for item in &mut tile.items {
                item.attrs.teleport = None;
            }
        }
    }
    for tag in daily.variants.active().flat_map(|v| &v.tags) {
        let (x, y, z) = tag.position;
        if sector.floor != -z {
            continue;
        }
        if let Some(tile) = sector.tiles.iter_mut().find(|t| t.x == x && t.y == y) {
            if tile.house != 0 {
                return Err(refusal("variant tag overlaps a house"));
            }
            let entry = tile
                .items
                .iter_mut()
                .find(|i| {
                    i.depth == 0
                        && manifest
                            .palette
                            .get(i.palette as usize)
                            .and_then(|p| palette_appearance(&p.key))
                            == Some(tag.appearance)
                })
                .ok_or_else(|| refusal("variant return marker missing"))?;
            entry.attrs.action = tag.action;
            entry.attrs.unique = tag.unique;
        }
    }
    Ok(sector)
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
    let manifest = &projection.manifest;
    let mut missing: std::collections::BTreeSet<_> = manifest
        .world_changes
        .iter()
        .flat_map(|_| daily.variants.portals())
        .map(|p| p.position)
        .chain(projection.barriers())
        .collect();
    let mut projected = |sector: Sector| {
        let sector = project_sector(manifest, daily, sector)?;
        for tile in &sector.tiles {
            missing.remove(&(tile.x, tile.y, -sector.floor));
        }
        visitor(manifest, sector)
    };
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
                projected(extra)?;
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
        projected(sector)
    })?;
    visited.manifest = projection.manifest.clone();
    if let Some(pending) = pending {
        for sector in pending.into_values() {
            projected(sector)?;
        }
    }
    if !missing.is_empty() {
        return Err(refusal("variant portal anchor missing"));
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
