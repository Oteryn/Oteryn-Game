//! Goblin Merchant's two-floor winch, from Crystal's rafzane_elevator.lua.
use super::*;

pub struct RafzaneElevator {
    pub from: TilePos,
    pub to: TilePos,
    pub evacuation: TilePos,
    pub sound: &'static str,
    pub calling: bool,
    moves: Vec<QuestMapItemMove>,
}

impl RafzaneElevator {
    /// Check the landing against the post-batch stack, so the upper closed
    /// rail cannot reject the very ride which opens it.
    pub fn landing_walkable(
        &self,
        facts: &BundleFacts,
        base: &crate::map::WorldBase,
        position: TilePos,
    ) -> bool {
        let Some(tile) = base.tile(position.x, position.y, position.floor) else {
            return false;
        };
        if !tile.walkable() || facts.house_tile(position) {
            return false;
        }
        let mut entries = Vec::new();
        if view::compose_into(base, None, facts, position, &mut entries).is_err() {
            return false;
        }
        entries.retain(|entry| {
            !self
                .moves
                .iter()
                .any(|movement| movement.from == position && movement.entry == *entry)
        });
        for movement in &self.moves {
            if movement.to == position {
                let Some(after) = facts.movement_item_facts(movement.appearance) else {
                    return false;
                };
                entries.push(view::ComposedEntry {
                    facts: after,
                    ..movement.entry
                });
            }
        }
        entries
            .iter()
            .all(|entry| facts.override_blocks(entry.facts) == Some(false))
    }

    pub fn project(&self, facts: &BundleFacts, base: &crate::map::WorldBase) -> bool {
        facts.apply_quest_map_item_batch(base, &self.moves)
    }
}

impl BundleFacts {
    pub fn prepare_rafzane_elevator(
        &self,
        base: &crate::map::WorldBase,
        winch: TilePos,
        appearance: u16,
    ) -> Option<RafzaneElevator> {
        let upper = TilePos {
            x: 33051,
            y: 32099,
            floor: -6,
        };
        let lower = TilePos { floor: -7, ..upper };
        let home = match (appearance, winch) {
            (17940, p) if p == (TilePos { x: 33052, ..upper }) => upper,
            (17944, p) if p == (TilePos { x: 33052, ..lower }) => lower,
            _ => return None,
        };
        let mut entries = Vec::new();
        for position in [upper, lower] {
            if self.house_tile(position) {
                return None;
            }
            let mut tile = Vec::new();
            view::compose_into(base, None, self, position, &mut tile).ok()?;
            entries.extend(
                tile.into_iter()
                    .filter(|e| matches!(e.facts.appearance_id, 17938 | 17939 | 17942 | 17943))
                    .map(|entry| (position, position, entry, entry.facts.appearance_id)),
            );
        }
        // The retained map has one platform and two rails. Refuse malformed
        // scenery instead of publishing a partial source move.
        if entries.len() != 3 {
            return None;
        }
        let platforms: Vec<_> = entries
            .iter()
            .filter(|(_, _, e, _)| e.facts.appearance_id == 17938)
            .collect();
        let [(from, _, _, _)] = platforms.as_slice() else {
            return None;
        };
        let from = *from;
        let to = if from == upper { lower } else { upper };
        let calling = from != home;
        let moved_rail = if from == upper { 17939 } else { 17943 };
        for appearance in [17938, moved_rail] {
            let entry = entries
                .iter_mut()
                .find(|(_, at, _, id)| *at == from && *id == appearance)?;
            entry.1 = to;
        }
        let transforms = if from == upper {
            [(upper, 17939, 17942), (lower, 17939, 17943)]
        } else {
            [(upper, 17942, 17939), (upper, 17943, 17939)]
        };
        for (position, before, after) in transforms {
            let entry = entries
                .iter_mut()
                .find(|(_, at, _, id)| *at == position && *id == before)?;
            entry.3 = after;
        }
        let moves = entries
            .into_iter()
            .map(|(from, to, entry, appearance)| {
                let key = match entry.source {
                    view::EntrySource::Base { placement_key, .. } => placement_key,
                    view::EntrySource::Added(id) => id.get(),
                    _ => return None,
                };
                Some(QuestMapItemMove {
                    from,
                    to,
                    entry,
                    expected_revision: self.object_revision(key),
                    appearance,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(RafzaneElevator {
            from,
            to,
            calling,
            evacuation: TilePos {
                y: if from == upper { 32098 } else { 32100 },
                ..from
            },
            sound: if from == upper { "Srrrt!" } else { "Zrrrt!" },
            moves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};

    #[test]
    fn rafzane_native_winch_moves_both_floors_and_rejects_replayed_batch() -> Result<(), String> {
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_states(
            ("oteryn:item.tibia.i17938", 0),
            ("oteryn:item.tibia.i17939", 1),
            &[
                ("oteryn:item.tibia.i17942", 2),
                ("oteryn:item.tibia.i17943", 3),
            ],
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| e.to_string())?;
        // Like the real B3 palette, only the inactive rails are placed. The
        // active rail must resolve from served content, not a map placement.
        data.manifest.palette.remove(4);
        data.manifest.world.min_x = 33024;
        data.manifest.world.max_x = 33056;
        data.manifest.world.min_y = 32096;
        data.manifest.world.max_y = 32128;
        data.manifest.world.floors = vec![-7, -6];
        let tile = |palettes: &[u32]| Tile {
            x: 33051,
            y: 32099,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: palettes
                .iter()
                .map(|palette| Item {
                    palette: *palette,
                    depth: 0,
                    attrs: Attrs::default(),
                })
                .collect(),
        };
        data.sectors = [-7, -6]
            .into_iter()
            .map(|floor| oteryn_world_bundle::bundle::Sector {
                floor,
                sx: 1032,
                sy: 1003,
                tiles: vec![if floor == -7 {
                    tile(&[0, 5, 3])
                } else {
                    tile(&[0, 4])
                }],
            })
            .collect();
        let bytes =
            oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &Default::default())
                .map_err(|e| e.to_string())?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(|e| e.to_string())?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(|e| e.to_string())?;
        let mut facts = BundleFacts::palette(&data.manifest, |key| {
            let id = key
                .strip_prefix("oteryn:item.tibia.i")?
                .parse::<u32>()
                .ok()?;
            Some(ItemDefinition {
                reference: NonZeroU32::new(match id {
                    17938 => 1,
                    17939 => 2,
                    17942 => 3,
                    17943 => 4,
                    _ => return None,
                })?,
                solid: Some(id == 17942),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        })
        .map_err(|e| format!("{e:?}"))?;
        for sector in &data.sectors {
            facts.push(&data.manifest, sector, &mut BTreeSet::new());
        }
        facts.finish();
        let winch = TilePos {
            x: 33052,
            y: 32099,
            floor: -6,
        };
        let rising = facts
            .prepare_rafzane_elevator(&base, winch, 17940)
            .ok_or("native lower platform")?;
        assert!(rising.calling);
        assert_eq!(rising.evacuation.y, 32100);
        assert_eq!(rising.sound, "Zrrrt!");
        assert!(rising.landing_walkable(&facts, &base, rising.to));
        assert!(
            facts.quest_callback_blocked(&base, rising.to, true),
            "the closed upper rail blocks before the batch"
        );
        assert!(rising.project(&facts, &base));
        assert!(
            !facts.quest_callback_blocked(&base, rising.to, true),
            "the committed active rail clears the upper collision"
        );
        assert!(
            !rising.project(&facts, &base),
            "stale scenery cannot move twice"
        );
        let descending = facts
            .prepare_rafzane_elevator(&base, winch, 17940)
            .ok_or("upper platform")?;
        assert!(!descending.calling);
        assert_eq!(descending.evacuation.y, 32098);
        assert_eq!(descending.sound, "Srrrt!");
        assert!(descending.project(&facts, &base));
        assert!(
            facts.quest_callback_blocked(&base, descending.from, true),
            "the departed upper platform closes its rail"
        );
        let back = facts
            .prepare_rafzane_elevator(&base, TilePos { floor: -7, ..winch }, 17944)
            .ok_or("lower winch")?;
        assert!(!back.calling);
        assert_eq!(back.from.floor, -7);
        assert!(
            facts
                .prepare_rafzane_elevator(&base, winch, 17944)
                .is_none()
        );
        Ok(())
    }
}
