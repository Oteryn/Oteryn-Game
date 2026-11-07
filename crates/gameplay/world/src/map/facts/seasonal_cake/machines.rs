use super::*;
impl BundleFacts {
    /// Crystal startup/tables/tile.lua:1114-1151 names all ten extraction cells.
    /// Eight already carry8886 in the authored base. Only the two absent Liberty
    /// pads need this seasonal overlay; cleanup restores their exact source stack.
    pub fn project_cake_machines(
        &self,
        base: &super::super::super::WorldBase,
        active: bool,
    ) -> bool {
        let Some(mut machine) = self.movement_item_facts(8886) else {
            return false;
        };
        machine.terrain_kind = Some(TerrainKind::Ground);
        let mut grounds = Vec::with_capacity(2);
        for y in 32801..=32802 {
            let pos = TilePos {
                x: 32377,
                y,
                floor: -7,
            };
            if self.house_tile(pos) {
                return false;
            }
            let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
                return false;
            };
            let Some((ground_ordinal, _)) = tile.ground() else {
                return false;
            };
            let mut ordinal = 0;
            for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
                if depth != 0 {
                    continue;
                }
                let Some(key) = placement_key(pos, ordinal) else {
                    return false;
                };
                let is_ground = ordinal == ground_ordinal;
                ordinal += 1;
                let palette = self
                    .placements
                    .binary_search_by_key(&key, |(k, _)| *k)
                    .ok()
                    .map(|i| self.placements[i].1.palette)
                    .or_else(|| self.unique_index(id));
                let Some(original) = palette.map(|i| self.palette[i as usize].facts) else {
                    return false;
                };
                if is_ground {
                    if original.appearance_id != if y == 32801 { 4517 } else { 4522 } {
                        return false;
                    }
                    grounds.push((key, pos, original));
                } else {
                    if y != 32801 || original.appearance_id != 3654 {
                        return false;
                    }
                    // Preserve the authored non-blocking decoration3654.
                }
            }
        }
        if grounds.len() != 2 {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if !states.has_capacity(grounds.len())
            || grounds
                .iter()
                .any(|(key, _, _)| states.changed.get(key).is_some_and(|(r, _)| *r == u64::MAX))
        {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        for (key, pos, original) in grounds {
            let target = if active { machine } else { original };
            if states
                .changed
                .get(&key)
                .is_none_or(|(_, facts)| *facts != target)
            {
                let revision = states
                    .changed
                    .get(&key)
                    .map_or(Some(1), |(r, _)| r.checked_add(1));
                let Some(revision) = revision else {
                    return false;
                };
                states.changed.insert(key, (revision, target));
            }
            if active {
                states.passable.insert(key, true);
                states.cake_ground_speeds.insert(pos, 150);
            } else {
                states.passable.remove(&key);
                states.cake_ground_speeds.remove(&pos);
            }
        }
        true
    }
}
