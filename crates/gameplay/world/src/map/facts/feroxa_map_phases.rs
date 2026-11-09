//! M52: atomic Channel-local scenery snapshots for the retained full-moon maps.
use super::*;
use crate::content::bundle_spell_tiles::{BundleTilePhases, tile_pos};
use crate::content::{LogicalCell, QualifiedSpellTile, SpellTileLookupError};

/// Physical facts resolved from the admitted phase palette, never portable custody.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeroxaPhaseTile {
    pub position: TilePos,
    pub entries: Vec<EntryFacts>,
    pub spell: QualifiedSpellTile,
    pub no_logout: bool,
    pub teleport: Option<(u8, TilePos)>,
}

#[derive(Debug)]
pub(super) struct FeroxaMapPhase {
    firing: [u8; 16],
    phase: u8,
    source: [u8; 32],
    revision: u64,
    tiles: BTreeMap<TilePos, FeroxaPhaseTile>,
    footprint: usize,
}

pub(super) struct PhaseTiles(Arc<Mutex<Arc<CallbackStates>>>);
impl BundleTilePhases for PhaseTiles {
    fn lookup(
        &self,
        cell: LogicalCell,
    ) -> Result<Option<QualifiedSpellTile>, SpellTileLookupError> {
        let pos = tile_pos(cell).ok_or(SpellTileLookupError::Absent)?;
        self.0
            .lock()
            .map_err(|_| SpellTileLookupError::Unknown)?
            .phase_spell(pos)
    }
}

impl BundleFacts {
    pub(crate) fn phase_spell_owner(&self) -> Arc<dyn BundleTilePhases> {
        Arc::new(PhaseTiles(Arc::clone(&self.callbacks)))
    }

    /// Called under the current Channel owner after its fenced World phase read.
    /// A replay cannot republish, substitute a payload, change firing or rewind.
    pub(crate) fn publish_feroxa_phase(
        &self,
        firing: [u8; 16],
        phase: u8,
        source: [u8; 32],
        tiles: Vec<FeroxaPhaseTile>,
    ) -> bool {
        if crate::foundation::WorldId::decode(&firing).is_err()
            || !matches!(phase, 1 | 2)
            || source == [0; 32]
            || tiles.is_empty()
            || tiles.len() > 1225
        {
            return false;
        }
        let mut prepared = BTreeMap::new();
        let mut footprint = 0;
        for tile in tiles {
            let p = tile.position;
            if !(33413..=33447).contains(&p.x)
                || !(31520..=31554).contains(&p.y)
                || p.floor != -11
                || self.house_tile(p)
                || tile.entries.is_empty()
                || tile.entries.len() > 64
                || tile
                    .entries
                    .iter()
                    .any(|entry| entry.pickupable || entry.count != 1 || entry.sub_type != 0)
                || tile.spell.flags().floor_change
            {
                return false;
            }
            let ground = tile
                .entries
                .iter()
                .find(|entry| entry.terrain_kind == Some(TerrainKind::Ground));
            let tops: Vec<_> = tile
                .entries
                .iter()
                .filter(|entry| entry.terrain_kind != Some(TerrainKind::Ground))
                .map(|entry| u32::from(entry.appearance_id))
                .collect();
            if ground.map(|entry| u32::from(entry.appearance_id)) != tile.spell.ground_source_id()
                || tops != tile.spell.top_source_ids()
                || tile
                    .entries
                    .iter()
                    .filter(|e| e.terrain_kind == Some(TerrainKind::Ground))
                    .count()
                    > 1
                || (ground.is_none() && !tile.spell.flags().block_solid)
            {
                return false;
            }
            if let Some((ordinal, destination)) = tile.teleport
                && (p
                    != (TilePos {
                        x: 33430,
                        y: 31537,
                        floor: -11,
                    })
                    || destination
                        != (TilePos {
                            x: 33457,
                            y: 31531,
                            floor: -11,
                        })
                    || tile
                        .entries
                        .get(usize::from(ordinal))
                        .is_none_or(|e| e.appearance_id != 1949 || !e.bound))
            {
                return false;
            }
            footprint += 1 + tile.entries.len();
            if prepared.insert(p, tile).is_some() {
                return false;
            }
        }
        let rotations = match self.rotations.lock() {
            Ok(r) => Arc::clone(&r),
            Err(_) => return false,
        };
        let Ok(mut state) = self.callbacks.lock() else {
            return false;
        };
        if let Some(previous) = &state.feroxa_phase {
            if previous.firing != firing || previous.phase > phase {
                return false;
            }
            if previous.phase == phase {
                return previous.source == source && previous.tiles == prepared;
            }
            if !previous.tiles.keys().eq(prepared.keys()) {
                return false;
            }
        }
        if !state.has_phase_capacity(footprint) {
            return false;
        }
        let inside =
            |key: &u64| view::placement_of(*key).is_some_and(|(p, _)| prepared.contains_key(&p));
        let revision = state
            .changed
            .iter()
            .filter(|(key, _)| inside(key))
            .map(|(_, (r, _))| *r)
            .chain(
                rotations
                    .iter()
                    .filter(|(key, _)| inside(key))
                    .map(|(_, (r, _))| *r),
            )
            .chain(state.feroxa_phase.iter().map(|p| p.revision))
            .max()
            .unwrap_or(0)
            .checked_add(1);
        let Some(revision) = revision else {
            return false;
        };
        let state = Arc::make_mut(&mut state);
        state.clear_feroxa_scenery(|p| prepared.contains_key(&p));
        state.feroxa_phase = Some(Arc::new(FeroxaMapPhase {
            firing,
            phase,
            source,
            revision,
            tiles: prepared,
            footprint,
        }));
        true
    }
}

impl CallbackStates {
    fn has_phase_capacity(&self, additional: usize) -> bool {
        self.has_capacity(additional)
    }

    pub(super) fn phase_revision(&self, key: u64) -> Option<u64> {
        let (pos, _) = view::placement_of(key)?;
        let phase = self.feroxa_phase.as_ref()?;
        phase.tiles.contains_key(&pos).then_some(phase.revision)
    }

    pub(super) fn phase_entry(&self, pos: TilePos, ordinal: u8) -> Option<Option<EntryFacts>> {
        let phase = self.feroxa_phase.as_ref()?;
        Some(
            phase
                .tiles
                .get(&pos)?
                .entries
                .get(usize::from(ordinal))
                .copied(),
        )
    }

    pub(super) fn phase_composition(&self, pos: TilePos) -> Option<view::ComposedTile> {
        let tile = self.feroxa_phase.as_ref()?.tiles.get(&pos)?;
        let entries = tile
            .entries
            .iter()
            .enumerate()
            .filter_map(|(ordinal, original)| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(pos, ordinal)?;
                if self.hidden.contains(&key) {
                    return None;
                }
                let facts = self
                    .changed
                    .get(&key)
                    .map_or(*original, |(_, after)| *after);
                Some(view::ComposedEntry {
                    source: view::EntrySource::Base {
                        ordinal,
                        placement_key: key,
                        movable: false,
                    },
                    facts,
                })
            })
            .collect();
        Some(view::ComposedTile {
            pos,
            entries,
            ground_speed: tile.spell.ground_speed().unwrap_or(0),
        })
    }

    pub(super) fn phase_spell(
        &self,
        pos: TilePos,
    ) -> Result<Option<QualifiedSpellTile>, SpellTileLookupError> {
        let Some(tile) = self.feroxa_phase.as_ref().and_then(|p| p.tiles.get(&pos)) else {
            return Ok(None);
        };
        if self.phase_has_dynamic_blocker(pos) {
            return Err(SpellTileLookupError::Unknown);
        }
        let first = placement_key(pos, 0).ok_or(SpellTileLookupError::Unknown)?;
        if self.hidden.range(first..=first + 63).next().is_some() {
            return Err(SpellTileLookupError::Unknown);
        }
        if self.changed.range(first..=first + 63).next().is_none() {
            return Ok(Some(tile.spell.clone()));
        }
        let mut tops = Vec::new();
        for (ordinal, before) in tile.entries.iter().enumerate() {
            let key = placement_key(pos, ordinal as u8).ok_or(SpellTileLookupError::Unknown)?;
            let after = self.changed.get(&key).map_or(*before, |(_, entry)| *entry);
            // The source portal's1949->22761 handoff changes identity and routing,
            // but neither object changes the independently qualified tile flags.
            if after != *before
                && !(before.appearance_id == 1949
                    && after.appearance_id == 22761
                    && pos
                        == (TilePos {
                            x: 33430,
                            y: 31537,
                            floor: -11,
                        })
                    && after.bound
                    && !after.pickupable
                    && after.count == 1
                    && after.blocks_projectile == before.blocks_projectile
                    && after.sub_type == before.sub_type
                    && (after.terrain_kind == Some(TerrainKind::Ground))
                        == (before.terrain_kind == Some(TerrainKind::Ground)))
            {
                return Err(SpellTileLookupError::Unknown);
            }
            if before.terrain_kind != Some(TerrainKind::Ground) {
                tops.push(u32::from(after.appearance_id));
            }
        }
        Ok(Some(QualifiedSpellTile::from_bundle(
            tile.spell.ground_source_id().zip(tile.spell.ground_speed()),
            tops,
            tile.spell.flags(),
            tile.spell.no_pvp_zone().zip(tile.spell.pvp_zone()),
        )))
    }
}

impl FeroxaMapPhase {
    pub(super) fn footprint(&self) -> usize {
        self.footprint
    }
}

impl BundleFacts {
    pub(crate) fn phase_contains(&self, pos: TilePos) -> bool {
        self.callbacks.lock().is_ok_and(|s| {
            s.feroxa_phase
                .as_ref()
                .is_some_and(|p| p.tiles.contains_key(&pos))
        })
    }
    pub fn phase_zones(&self, pos: TilePos) -> Result<Option<(bool, bool)>, view::ViewError> {
        let s = self
            .callbacks
            .lock()
            .map_err(|_| view::ViewError::MissingFacts)?;
        Ok(s.feroxa_phase
            .as_ref()
            .and_then(|p| p.tiles.get(&pos))
            .map(|tile| (tile.no_logout, tile.spell.flags().protection_zone)))
    }
    pub(crate) fn phase_teleport(&self, pos: TilePos) -> Option<Option<TilePos>> {
        let s = self.callbacks.lock().ok()?;
        let tile = s.feroxa_phase.as_ref()?.tiles.get(&pos)?;
        Some(tile.teleport.and_then(|(ordinal, to)| {
            let key = placement_key(pos, ordinal)?;
            let current = s
                .changed
                .get(&key)
                .map_or(tile.entries.get(usize::from(ordinal))?, |(_, e)| e);
            (!s.hidden.contains(&key) && current.appearance_id == 1949).then_some(to)
        }))
    }
}

#[cfg(test)]
mod tests;
