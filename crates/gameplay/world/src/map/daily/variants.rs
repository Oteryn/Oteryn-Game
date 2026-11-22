//! Physical projection of the same committed choices used by the admission owner.
use super::*;
use crate::map::overlay::TilePos;
use oteryn_gameplay_world_change::variants::{self, Definition, Portal, Spawn, Variant};

#[derive(Debug, Default)]
pub(crate) struct VariantProjection {
    definitions: Vec<(&'static Definition, Option<&'static Variant>)>,
    portals: BTreeMap<variants::Position, Option<&'static Portal>>,
    barriers: std::collections::BTreeSet<TilePos>,
    homes: std::collections::BTreeSet<(i32, i32, i16)>,
}

impl VariantProjection {
    pub(super) fn new(manifest: &Manifest, daily: &DailyMap) -> Result<Self, Error> {
        let selected = daily.variants;
        if !selected.validate() {
            return Err(refusal("incompatible daily variant decision"));
        }
        let mut result = Self::default();
        if !selected.installed() || manifest.projection_class != "server" {
            return Ok(result);
        }

        for definition in variants::definitions().map_err(refusal)? {
            if daily
                .unavailable_variants
                .contains(&definition.key.as_str())
            {
                continue;
            }
            let positions = definition
                .inactive_tiles
                .iter()
                .map(|tile| tile.position)
                .chain(definition.variants.iter().flat_map(|variant| {
                    variant
                        .tiles
                        .iter()
                        .map(|tile| tile.position)
                        .chain(variant.spawns.iter().map(|spawn| spawn.position))
                        .chain(variant.npcs.iter().map(|npc| npc.position))
                        .chain(variant.barriers.iter().copied())
                        .chain(
                            variant
                                .portals
                                .iter()
                                .flat_map(|portal| [portal.position, portal.destination]),
                        )
                }));
            // A regional bundle cannot host only one end of a passage or part of a roster.
            if !positions.into_iter().all(|(x, y, z)| {
                (manifest.world.min_x..manifest.world.max_x).contains(&x)
                    && (manifest.world.min_y..manifest.world.max_y).contains(&y)
                    && manifest.world.floors.contains(&-z)
            }) {
                continue;
            }
            result.homes.extend(
                definition
                    .variants
                    .iter()
                    .flat_map(|v| &v.spawns)
                    .map(|spawn| {
                        let (x, y, z) = spawn.position;
                        (i32::from(x), i32::from(y), i16::from(z))
                    }),
            );
            let variant = selected.variant(definition);
            result.definitions.push((definition, variant));
            for portal in definition.variants.iter().flat_map(|v| &v.portals) {
                result
                    .portals
                    .entry(portal.position)
                    .or_insert_with(|| selected.passage(portal.position).map(|(_, chosen)| chosen));
            }
            if let Some(variant) = variant {
                result
                    .barriers
                    .extend(
                        variant
                            .barriers
                            .iter()
                            .map(|&(x, y, z)| TilePos { x, y, floor: -z }),
                    );
            }
        }
        Ok(result)
    }

    pub(super) fn tiles(&self) -> impl Iterator<Item = &variants::Tile> {
        self.definitions.iter().flat_map(|(definition, variant)| {
            definition
                .inactive_tiles
                .iter()
                .chain(variant.iter().flat_map(|v| &v.tiles))
        })
    }

    pub(super) fn appearances(&self) -> impl Iterator<Item = u16> {
        self.tiles()
            .flat_map(|tile| tile.entries.iter().map(|entry| entry.0))
            .chain(
                self.portals
                    .values()
                    .flatten()
                    .map(|portal| portal.appearance),
            )
    }

    pub(crate) fn barrier(&self, position: TilePos) -> bool {
        self.barriers.contains(&position)
    }

    pub(crate) fn passages(&self) -> impl Iterator<Item = variants::Position> + '_ {
        self.portals
            .iter()
            .filter_map(|(at, portal)| portal.map(|_| *at))
    }

    pub(super) fn apply_portals(
        &self,
        manifest: &Manifest,
        sector: &mut Sector,
    ) -> Result<(), Error> {
        for (&(x, y, z), portal) in &self.portals {
            if (sector.floor, sector.sx, sector.sy) != (-z, x / 32, y / 32) {
                continue;
            }
            let tile = sector
                .tiles
                .iter_mut()
                .find(|tile| (tile.x, tile.y) == (x, y));
            let Some(tile) = tile else {
                if portal.is_some() {
                    return Err(refusal("daily variant portal ground missing"));
                }
                continue;
            };
            if tile.house != 0 {
                return Err(refusal("daily variant portal overlaps a house"));
            }
            for entry in tile.items.iter_mut().filter(|entry| entry.depth == 0) {
                entry.attrs.teleport = None;
            }
            let Some(portal) = portal else { continue };
            let palette = index(manifest, portal.appearance)?;
            let attrs = Attrs {
                action: portal.action,
                unique: portal.unique,
                ..Attrs::default()
            };
            if let Some(existing) = tile
                .items
                .iter_mut()
                .find(|item| item.depth == 0 && item.palette == palette)
            {
                existing.attrs.action = attrs.action;
                existing.attrs.unique = attrs.unique;
            } else {
                tile.items.push(Item {
                    palette,
                    depth: 0,
                    attrs,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn owns_npc(&self, key: &str) -> bool {
        self.definitions
            .iter()
            .flat_map(|(d, _)| &d.variants)
            .flat_map(|v| &v.npcs)
            .any(|npc| npc.key == key)
    }

    pub(crate) fn npcs(&self) -> impl Iterator<Item = (&'static str, variants::Position)> + '_ {
        self.definitions
            .iter()
            .filter_map(|(_, variant)| *variant)
            .flat_map(|variant| &variant.npcs)
            .map(|npc| (npc.key.as_str(), npc.position))
    }

    pub(crate) fn spawns(&self) -> impl Iterator<Item = (&str, &[Spawn])> {
        self.definitions.iter().filter_map(|(definition, variant)| {
            variant.map(|v| (definition.key.as_str(), v.spawns.as_slice()))
        })
    }

    pub(crate) fn owns_spawn(&self, position: (i32, i32, i16)) -> bool {
        self.homes.contains(&position)
    }
}
