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

/// Resolve every source ID through the served generation before any publication.
/// The caller supplies only definition identity/kind; physical flags come from
/// the independently pinned source item loader, not a stale bundle palette.
pub fn prepare_feroxa_phase(
    elapsed_ms: u64,
    mut resolve: impl FnMut(u16) -> Option<EntryFacts>,
) -> Option<(u8, [u8; 32], Vec<FeroxaPhaseTile>)> {
    let maps = crate::content::feroxa_source_maps::FeroxaSourceMaps::embedded()?;
    let phase = maps.due(elapsed_ms)?;
    let mut tiles = Vec::with_capacity(phase.tiles.len());
    for source in &phase.tiles {
        let position = TilePos {
            x: source.0,
            y: source.1,
            floor: -i8::try_from(source.2).ok()?,
        };
        let mut entries = Vec::with_capacity(source.4.len());
        let teleport = phase
            .teleports
            .iter()
            .find(|p| p[..3] == [source.0, source.1, u16::from(source.2)]);
        for (ordinal, id) in source.4.iter().enumerate() {
            let physical = maps.item(*id)?;
            let mut entry = resolve(*id)?;
            if entry.appearance_id != *id
                || physical.pickupable
                || physical.container
                || physical.floor_change
            {
                return None;
            }
            if physical.ground {
                entry.terrain_kind = Some(TerrainKind::Ground);
            }
            entry.blocks_projectile = physical.block_projectile;
            entry.pickupable = false;
            entry.bound = teleport.is_some_and(|p| usize::from(p[3]) == ordinal);
            entry.count = 1;
            entry.sub_type = 0;
            entries.push(entry);
        }
        tiles.push(FeroxaPhaseTile {
            position,
            entries,
            spell: maps.spell_tile(source)?,
            no_logout: source.3 & 8 != 0,
            teleport: match teleport {
                Some(p) => Some((
                    u8::try_from(p[3]).ok()?,
                    TilePos {
                        x: p[4],
                        y: p[5],
                        floor: -i8::try_from(p[6]).ok()?,
                    },
                )),
                None => None,
            },
        });
    }
    Some((phase.phase, maps.capture_digest(), tiles))
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
    fn scenery_tile(&self, pos: TilePos) -> Option<(u64, &FeroxaPhaseTile)> {
        if let Some((revision, tile)) = self.habitat_tiles.get(&pos) {
            return Some((*revision, tile));
        }
        let phase = self.feroxa_phase.as_ref()?;
        Some((phase.revision, phase.tiles.get(&pos)?))
    }

    fn has_phase_capacity(&self, additional: usize) -> bool {
        self.has_capacity(additional)
    }

    pub(super) fn phase_revision(&self, key: u64) -> Option<u64> {
        let (pos, _) = view::placement_of(key)?;
        self.scenery_tile(pos).map(|(revision, _)| revision)
    }

    pub(super) fn phase_entry(&self, pos: TilePos, ordinal: u8) -> Option<Option<EntryFacts>> {
        let (_, tile) = self.scenery_tile(pos)?;
        Some(tile.entries.get(usize::from(ordinal)).copied())
    }

    pub(super) fn phase_composition(&self, pos: TilePos) -> Option<view::ComposedTile> {
        let (_, tile) = self.scenery_tile(pos)?;
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
                        openable_source: false,
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
        let Some((_, tile)) = self.scenery_tile(pos) else {
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
        self.callbacks
            .lock()
            .is_ok_and(|s| s.scenery_tile(pos).is_some())
    }
    pub fn phase_zones(&self, pos: TilePos) -> Result<Option<(bool, bool)>, view::ViewError> {
        let s = self
            .callbacks
            .lock()
            .map_err(|_| view::ViewError::MissingFacts)?;
        Ok(s.scenery_tile(pos)
            .map(|(_, tile)| (tile.no_logout, tile.spell.flags().protection_zone)))
    }
    pub(crate) fn phase_teleport(&self, pos: TilePos) -> Option<Option<TilePos>> {
        let s = self.callbacks.lock().ok()?;
        let (_, tile) = s.scenery_tile(pos)?;
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

impl BundleFacts {
    /// Swap a habitat to the retained counterpart's scenery. Both reads use
    /// the immutable base so changing the other habitat cannot alter the source.
    pub fn publish_habitat_mirror(
        &self,
        base: &crate::map::WorldBase,
        spells: &crate::content::bundle_spell_tiles::BundleSpellTiles,
        origin: TilePos,
        counterpart_x: u16,
    ) -> bool {
        if !matches!((origin.x, counterpart_x), (33611, 33634) | (33634, 33611))
            || !matches!(origin.y, 32658 | 32676 | 32694 | 32712)
            || origin.floor != -12
        {
            return false;
        }
        let mut tiles = BTreeMap::new();
        for dx in 0..15 {
            for dy in 0..16 {
                let position = TilePos {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    floor: origin.floor,
                };
                let source = TilePos {
                    x: counterpart_x + dx,
                    ..position
                };
                if self.house_tile(position) || self.house_tile(source) {
                    return false;
                }
                let (Some(tile), Some(previous)) = (
                    base.tile(source.x, source.y, source.floor),
                    base.tile(position.x, position.y, position.floor),
                ) else {
                    return false;
                };
                let logical = |p: TilePos| LogicalCell {
                    x: i32::from(p.x),
                    y: i32::from(p.y),
                    z: -i32::from(p.floor),
                };
                let Ok(source_spell) = spells.lookup_base(logical(source)) else {
                    return false;
                };
                let Ok(target_spell) = spells.lookup_base(logical(position)) else {
                    return false;
                };
                if source_spell.flags().floor_change || target_spell.flags().floor_change {
                    return false;
                }
                let rotations = Default::default();
                let mut entries = Vec::new();
                for (ordinal, (&id, &depth)) in tile.ids().iter().zip(tile.depths()).enumerate() {
                    let Ok(ordinal) = u8::try_from(ordinal) else {
                        return false;
                    };
                    let Some(entry) = self.entry(source, ordinal, id, None, &rotations) else {
                        return false;
                    };
                    if depth != 0 || entry.pickupable || entry.bound || entry.count != 1 {
                        return false;
                    }
                    entries.push(entry);
                }
                for (ordinal, (&id, &depth)) in
                    previous.ids().iter().zip(previous.depths()).enumerate()
                {
                    let Ok(ordinal) = u8::try_from(ordinal) else {
                        return false;
                    };
                    let Some(entry) = self.entry(position, ordinal, id, None, &rotations) else {
                        return false;
                    };
                    if depth != 0 || entry.pickupable || entry.bound {
                        return false;
                    }
                }
                // Crystal's desert and ice scripts account for these retained
                // physical palette deltas from the counterpart scenery.
                let replacements: &[u16] = match (position.x, position.y) {
                    (33642, 32683) | (33643, 32682) => &[417, 12516],
                    (33636, 32702) | (33638, 32705) | (33639, 32704) => &[6598],
                    (33637, 32702) => &[6607],
                    (33638, 32702 | 32703) => &[6601],
                    (33638, 32704) | (33639, 32701 | 32702) => &[6594],
                    (33639, 32705) => &[6597],
                    (33640, 32705) => &[6608],
                    _ => &[],
                };
                if !replacements.is_empty() {
                    let Some(replaced) = replacements
                        .iter()
                        .map(|id| self.movement_item_facts(*id))
                        .collect::<Option<Vec<_>>>()
                    else {
                        return false;
                    };
                    if replaced.iter().any(|e| e.pickupable || e.bound) {
                        return false;
                    }
                    entries = replaced;
                }
                let ground = entries
                    .iter()
                    .find(|e| e.terrain_kind == Some(TerrainKind::Ground));
                let Some(ground) = ground else { return false };
                let tops = entries
                    .iter()
                    .filter(|e| e.terrain_kind != Some(TerrainKind::Ground))
                    .map(|e| u32::from(e.appearance_id))
                    .collect();
                let mut flags = source_spell.flags();
                flags.protection_zone = target_spell.flags().protection_zone;
                let spell = QualifiedSpellTile::from_bundle(
                    source_spell
                        .ground_speed()
                        .map(|speed| (u32::from(ground.appearance_id), speed)),
                    tops,
                    flags,
                    target_spell.no_pvp_zone().zip(target_spell.pvp_zone()),
                );
                tiles.insert(
                    position,
                    FeroxaPhaseTile {
                        position,
                        entries,
                        spell,
                        no_logout: false,
                        teleport: None,
                    },
                );
            }
        }
        let Ok(mut state) = self.callbacks.lock() else {
            return false;
        };
        let inside = |key| placement_key_owner(key).is_some_and(|pos| tiles.contains_key(&pos));
        if !state.has_capacity(tiles.values().map(|tile| 1 + tile.entries.len()).sum())
            || state.reserved.iter().copied().any(inside)
            || state
                .reserved_positions
                .iter()
                .any(|pos| tiles.contains_key(pos))
            || state
                .movement_items
                .keys()
                .any(|pos| tiles.contains_key(pos))
        {
            return false;
        }
        let revisions = state
            .habitat_tiles
            .values()
            .map(|(revision, _)| *revision)
            .chain(
                state
                    .changed
                    .iter()
                    .filter(|(key, _)| inside(**key))
                    .map(|(_, (revision, _))| *revision),
            );
        let Some(revision) = revisions.max().unwrap_or(0).checked_add(1) else {
            return false;
        };
        let state = Arc::make_mut(&mut state);
        state.clear_feroxa_scenery(|pos| tiles.contains_key(&pos));
        state
            .habitat_tiles
            .extend(tiles.into_iter().map(|(pos, tile)| (pos, (revision, tile))));
        true
    }
}

fn placement_key_owner(key: u64) -> Option<TilePos> {
    view::placement_of(key).map(|(pos, _)| pos)
}
