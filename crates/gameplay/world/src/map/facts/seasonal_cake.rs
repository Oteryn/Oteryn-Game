mod machines;
use super::*;
impl BundleFacts {
    /// Crystal Cake config:257-299 clears all source entries in forty floor and
    /// sixteen rail cells, preserving their identities for event cleanup.
    /// The current Channel owner publishes this bounded projection atomically.
    pub fn project_cake_bridge(&self, base: &super::super::WorldBase, open: bool) -> bool {
        let (Some(bridge), Some(mut rail)) = (
            self.movement_item_facts(48227),
            self.movement_item_facts(12113),
        ) else {
            return false;
        };
        if bridge.terrain_kind != Some(TerrainKind::Ground) {
            return false;
        }
        rail.terrain_kind = Some(TerrainKind::Ground);
        let mut grounds = Vec::with_capacity(56);
        let mut borders = Vec::new();
        for x in 33300..=33306 {
            for y in 32052..=32059 {
                let pos = TilePos { x, y, floor: -7 };
                if self.house_tile(pos) {
                    return false;
                }
                let Some(tile) = base.tile(x, y, -7) else {
                    return false;
                };
                let Some((ground_ordinal, _)) = tile.ground() else {
                    return false;
                };
                let mut ordinal = 0;
                let mut ground_count = 0;
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
                        if !matches!(original.appearance_id, 4515 | 4519 | 4527 | 4597..=4602) {
                            return false;
                        }
                        ground_count += 1;
                        grounds.push((
                            key,
                            original,
                            if x == 33300 || x == 33306 {
                                rail
                            } else {
                                bridge
                            },
                        ));
                    } else {
                        if !matches!(original.appearance_id, 4597..=4602 | 4633 | 4635) {
                            return false;
                        }
                        borders.push(key);
                    }
                }
                if ground_count != 1 {
                    return false;
                }
            }
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if !states.has_capacity(grounds.len() + borders.len())
            || grounds
                .iter()
                .any(|(k, _, _)| states.changed.get(k).is_some_and(|(r, _)| *r == u64::MAX))
        {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        for (key, original, target) in grounds {
            let revision = states.changed.get(&key).map_or(0, |(r, _)| *r) + 1;
            states
                .changed
                .insert(key, (revision, if open { target } else { original }));
            if open {
                states.passable.insert(key, true);
            } else {
                states.passable.remove(&key);
            }
        }
        for x in 33300..=33306 {
            for y in 32052..=32059 {
                let pos = TilePos { x, y, floor: -7 };
                if open {
                    states
                        .cake_ground_speeds
                        .insert(pos, if x == 33300 || x == 33306 { 100 } else { 150 });
                } else {
                    states.cake_ground_speeds.remove(&pos);
                }
            }
        }
        for key in borders {
            if open {
                states.hidden.insert(key);
            } else {
                states.hidden.remove(&key);
            }
        }
        true
    }

    /// Closed source tower projection. Completed upper floors vanish and a bite
    /// made in any Channel hides the exact same edible source placement everywhere.
    pub fn project_cake_tower(
        &self,
        base: &super::super::WorldBase,
        stage: u8,
        layer: u8,
        removed: &BTreeSet<(u16, u16)>,
    ) -> bool {
        if stage > 3 || !(1..=4).contains(&layer) || removed.len() > 1116 {
            return false;
        }
        let upper = layer - 1;
        let mut hidden = BTreeSet::new();
        let mut blocked = BTreeSet::new();
        for z in 4_u8..=7 {
            for x in 33287..=33330 {
                for y in 32065..=32103 {
                    let pos = TilePos {
                        x,
                        y,
                        floor: -(z as i8),
                    };
                    let Some(tile) = base.tile(x, y, pos.floor) else {
                        continue;
                    };
                    let ground = tile.ground().map(|(ordinal, _)| ordinal);
                    let mut ordinal = 0;
                    for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
                        if depth != 0 {
                            continue;
                        }
                        let Some(key) = placement_key(pos, ordinal) else {
                            return false;
                        };
                        let is_ground = ground == Some(ordinal);
                        ordinal += 1;
                        let palette = self
                            .placements
                            .binary_search_by_key(&key, |(k, _)| *k)
                            .ok()
                            .map(|i| self.placements[i].1.palette)
                            .or_else(|| self.unique_index(id));
                        let Some(facts) = palette.map(|i| self.palette[i as usize].facts) else {
                            return false;
                        };
                        let upper_removed = z < 7
                            && z < 4 + upper
                            && matches!(facts.appearance_id, 12147 | 12148 | 12152..=12155);
                        let eaten = z == 7
                            && removed.contains(&(x, y))
                            && matches!(facts.appearance_id, 12085 | 12086);
                        if upper_removed || eaten {
                            hidden.insert(key);
                            if is_ground {
                                blocked.insert(key);
                            }
                        }
                    }
                }
            }
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if !states.has_capacity(hidden.len() + blocked.len()) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        for key in std::mem::take(&mut states.cake_hidden) {
            if !states.gathering_hidden.contains(&key) {
                states.hidden.remove(&key);
            }
        }
        for key in std::mem::take(&mut states.cake_blocked) {
            states.passable.remove(&key);
            if let Some((pos, _)) = view::placement_of(key) {
                states.cake_ground_speeds.remove(&pos);
            }
        }
        states.hidden.extend(hidden.iter().copied());
        for key in &blocked {
            states.passable.insert(*key, false);
            if let Some((pos, _)) = view::placement_of(*key) {
                states.cake_ground_speeds.insert(pos, 0);
            }
        }
        states.cake_hidden = hidden;
        states.cake_blocked = blocked;
        true
    }
}
