//! Durable wooden bar replacements share the Channel's map and spell projection.
use super::quest_callbacks::CallbackStates;
use super::*;
use crate::content::{NativeSpellTileFlags, QualifiedSpellTile, SpellTileLookupError};
use crate::durability::terrain_tool::wooden_bar_remains;
use crate::map::boot::BundleMap;
use crate::map::spell_tiles::BundleSpellTiles;
use serde_json::Value;
use std::sync::OnceLock;

/// Collision facts qualified by the pinned Terrain and WorldObject catalogue records.
#[derive(Clone, Copy)]
pub struct WoodenBarCollision {
    pub terrain_kind: Option<TerrainKind>,
    pub block_solid: bool,
    pub blocks_projectile: bool,
}

pub fn wooden_bar_collision(appearance: u16) -> Option<WoodenBarCollision> {
    match appearance {
        2291..=2296 | 3145 | 3146 => pinned_collision(appearance),
        _ => None,
    }
}

fn pinned_collision(appearance: u16) -> Option<WoodenBarCollision> {
    static COLLISION: OnceLock<BTreeMap<u16, WoodenBarCollision>> = OnceLock::new();
    COLLISION
        .get_or_init(load_collision)
        .get(&appearance)
        .copied()
}

pub(crate) fn qualified_bar_tile_collision(key: &str) -> Option<WoodenBarCollision> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")
        .or_else(|| key.strip_prefix("oteryn:terrain.tibia.i"))?
        .parse::<u16>()
        .ok()?;
    match id {
        2291..=2296 | 7593 => pinned_collision(id),
        _ => None,
    }
}

fn load_collision() -> BTreeMap<u16, WoodenBarCollision> {
    let mut result = BTreeMap::new();
    for (family, prefix, bytes, appearances) in [
        (
            "Terrain",
            "terrain",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/terrain/terrain-00500-00999.json"
            ))
            .as_slice(),
            &[2291, 2292, 2293, 2294, 2295, 2296][..],
        ),
        (
            "Terrain",
            "terrain",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/terrain/terrain-02000-02499.json"
            ))
            .as_slice(),
            // The published 2295 bars at (33214, 31489, 7) stand on this ground.
            // Its pinned sight flag also supersedes the missing Item policy.
            &[7593][..],
        ),
        (
            "WorldObject",
            "world-object",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/objects/objects-00500-00999.json"
            ))
            .as_slice(),
            &[3145, 3146][..],
        ),
    ] {
        let Ok(shard) = serde_json::from_slice::<Value>(bytes) else {
            continue;
        };
        let Some(records) = shard["records"]
            .as_array()
            .filter(|_| shard["family"] == family)
        else {
            continue;
        };
        for &id in appearances {
            let identity = format!("oteryn:{prefix}.tibia.i{id}");
            let item = format!("oteryn:item.tibia.i{id}");
            let Some(record) = records
                .iter()
                .find(|row| row["identity"]["key"] == identity)
            else {
                continue;
            };
            let pointer = &record["provenance"]["item_pointer"];
            if pointer["family"] != "Item"
                || pointer["key"] != item
                || pointer["revision"] != "definition-r1"
            {
                continue;
            }
            let qualified = if family == "Terrain" {
                let (kind_name, kind) = if id == 7593 {
                    ("ground", TerrainKind::Ground)
                } else {
                    ("wall", TerrainKind::Wall)
                };
                if record["kind"]["state"] != "KNOWN" || record["kind"]["value"] != kind_name {
                    continue;
                }
                let Some(walkable) = known_bool(&record["walkable"]) else {
                    continue;
                };
                (Some(kind), !walkable, record)
            } else {
                let collision = &record["collision"];
                let Some(solid) = known_bool(&collision["blocks_movement"]) else {
                    continue;
                };
                (None, solid, collision)
            };
            // Crystal items.cpp:234 translates the appearance's unsight into blockProjectile.
            let Some(blocks_projectile) = known_bool(&qualified.2["blocks_projectile"])
                .or_else(|| known_bool(&qualified.2["blocks_sight"]))
            else {
                continue;
            };
            result.insert(
                id,
                WoodenBarCollision {
                    terrain_kind: qualified.0,
                    block_solid: qualified.1,
                    blocks_projectile,
                },
            );
        }
    }
    result
}

fn known_bool(field: &Value) -> Option<bool> {
    if field["state"] == "KNOWN" {
        field["value"].as_bool()
    } else {
        None
    }
}

fn is_bar_successor(original: u16, current: u16) -> bool {
    let current = u32::from(current);
    wooden_bar_remains(u32::from(original)).is_some_and(|(debris, restored)| {
        current == debris
            || current == restored
            || wooden_bar_remains(restored)
                .is_some_and(|(debris, restored)| current == debris || current == restored)
    })
}

impl BundleMap {
    /// Publish one committed revision into the view, movement and spell facts together.
    pub fn install_wooden_bar_state(&self, key: u64, revision: u64, mut after: EntryFacts) -> bool {
        let Some((pos, ordinal)) = view::placement_of(key) else {
            return false;
        };
        let Some(tile) = self.overlay.base().tile(pos.x, pos.y, pos.floor) else {
            return false;
        };
        if revision == 0
            || self.facts.terrain_tool_action(key) != Some(None)
            || !matches!(after.definition, MapDefinition::Item(_))
            || after.bound
            || after.pickupable
            || after.count != 1
            || after.sub_type != 0
            || !tile
                .appearance(ordinal)
                .is_some_and(|original| is_bar_successor(original, after.appearance_id))
        {
            return false;
        }
        let Some(collision) = wooden_bar_collision(after.appearance_id) else {
            return false;
        };
        after.terrain_kind = collision.terrain_kind;
        after.blocks_projectile = collision.blocks_projectile;
        let Ok(mut states) = self.facts.callbacks.lock() else {
            return false;
        };
        if let Some((current, facts)) = states.changed.get(&key)
            && *current >= revision
        {
            return *current > revision || *facts == after;
        }
        let stack = (!states.wooden_bar_spell_tiles.contains_key(&pos))
            .then(|| WoodenBarSpellTile::from_base(&self.facts, &self.spell_tiles, pos, tile));
        let additional = usize::from(!states.changed.contains_key(&key))
            + stack.as_ref().map_or(0, |stack| {
                stack.as_ref().map_or(1, WoodenBarSpellTile::footprint)
            });
        if states.key_reserved(key) || (additional != 0 && !states.has_capacity(additional)) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        states.changed.insert(key, (revision, after));
        states.passable.insert(key, !collision.block_solid);
        if let Some(stack) = stack {
            states
                .wooden_bar_spell_tiles
                .insert(pos, stack.map(Arc::new));
        }
        true
    }
}

#[derive(Clone, Debug)]
struct SpellEntry {
    key: u64,
    base: EntryFacts,
    solid: bool,
    immovable: [bool; 3],
}

#[derive(Clone, Debug)]
pub(super) struct WoodenBarSpellTile {
    base: QualifiedSpellTile,
    ground_ordinal: Option<usize>,
    ground_blocked: bool,
    entries: Vec<SpellEntry>,
}

impl WoodenBarSpellTile {
    fn from_base(
        facts: &BundleFacts,
        spells: &BundleSpellTiles,
        pos: TilePos,
        tile: crate::map::TileView<'_>,
    ) -> Option<Self> {
        let index = spells.base.tile_index(pos.x, pos.y, pos.floor)?;
        // An unqualified base remains unknown, even if one known obstruction breaks.
        let base = spells
            .shapes
            .get(*spells.tiles.get(index)? as usize)?
            .clone();
        let rotations = RotationStates::default();
        let empty_attrs = Attrs::default();
        let entries = tile
            .ids()
            .iter()
            .copied()
            .zip(tile.depths().iter().copied())
            .filter(|&(_, depth)| depth == 0)
            .enumerate()
            .map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = view::placement_key(pos, ordinal)?;
                let placement = facts
                    .placements
                    .binary_search_by_key(&key, |(key, _)| *key)
                    .ok()
                    .map(|i| &facts.placements[i].1);
                let palette = placement
                    .map(|p| p.palette)
                    .or_else(|| facts.unique_index(id))?;
                let base = facts.entry(pos, ordinal, id, None, &rotations)?;
                Some(SpellEntry {
                    key,
                    base,
                    solid: facts.blocks(palette),
                    immovable: crate::map::spell_tiles::immovable::flags(
                        u32::from(base.appearance_id),
                        placement.map_or(&empty_attrs, |p| &p.attrs),
                    ),
                })
            })
            .collect::<Option<_>>()?;
        Some(Self {
            base,
            ground_ordinal: tile.ground().map(|(ordinal, _)| usize::from(ordinal)),
            ground_blocked: tile.ground().is_some() && !tile.walkable(),
            entries,
        })
    }

    pub(super) fn footprint(&self) -> usize {
        1 + self.entries.len()
    }

    pub(super) fn lookup(
        &self,
        states: &CallbackStates,
        pos: TilePos,
        rotations: &RotationStates,
    ) -> Result<QualifiedSpellTile, SpellTileLookupError> {
        let unknown = SpellTileLookupError::Unknown;
        let first = view::placement_key(pos, 0).ok_or(unknown)?;
        if states.phase_has_dynamic_blocker(pos)
            || states.movement_items.contains_key(&pos)
            || states.hidden.range(first..=first + 63).next().is_some()
            || rotations.range(first..=first + 63).next().is_some()
        {
            return Err(unknown);
        }
        let mut tops = Vec::with_capacity(self.base.top_source_ids().len());
        let mut flags = NativeSpellTileFlags {
            block_solid: self.ground_blocked,
            block_projectile: false,
            immovable_block_solid: false,
            immovable_block_item: false,
            immovable_nonfield_block_item: false,
            ..self.base.flags()
        };
        for (ordinal, entry) in self.entries.iter().enumerate() {
            let (facts, solid, immovable) = if let Some((_, after)) = states.changed.get(&entry.key)
            {
                let collision = wooden_bar_collision(after.appearance_id).ok_or(unknown)?;
                if !is_bar_successor(entry.base.appearance_id, after.appearance_id)
                    || states.passable.get(&entry.key) != Some(&!collision.block_solid)
                    || after.bound
                {
                    return Err(unknown);
                }
                (
                    *after,
                    collision.block_solid,
                    crate::map::spell_tiles::immovable::flags(
                        u32::from(after.appearance_id),
                        &Attrs::default(),
                    ),
                )
            } else {
                (entry.base, entry.solid, entry.immovable)
            };
            flags.block_solid |= solid;
            flags.block_projectile |= facts.blocks_projectile;
            flags.immovable_block_solid |= immovable[0];
            flags.immovable_block_item |= immovable[1];
            flags.immovable_nonfield_block_item |= immovable[2];
            if self.ground_ordinal != Some(ordinal) {
                tops.push(u32::from(facts.appearance_id));
            }
        }
        Ok(QualifiedSpellTile::from_bundle(
            self.base.ground_source_id().zip(self.base.ground_speed()),
            tops,
            flags,
            self.base.no_pvp_zone().zip(self.base.pvp_zone()),
        ))
    }
}
