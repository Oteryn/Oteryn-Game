//! Crystal201189f a_piece_of_cake_config.lua:249-350,473-483,575-586.
//! The durable World phase selects presence; this is its Channel-local projection.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct BridgeProjection {
    tiles: BTreeMap<TilePos, BridgeTile>,
}

#[derive(Clone, Debug)]
struct BridgeTile {
    created: EntryId,
    hidden: Vec<u64>,
    previous_passable: Vec<(u64, Option<bool>)>,
    displaced: Vec<view::ComposedEntry>,
    speed: u16,
}

impl BridgeProjection {
    pub(super) fn ground_speed(&self, pos: TilePos) -> Option<u16> {
        self.tiles.get(&pos).map(|tile| tile.speed)
    }
}

impl BundleFacts {
    /// Source tiles must already exist; missing tiles are skipped by the donor.
    /// Both the central floor and the decorative side tiles are passable ground.
    pub fn project_cake_bridge(
        &self,
        base: &crate::map::WorldBase,
        present: bool,
        floor: EntryFacts,
        rail: EntryFacts,
    ) -> bool {
        if floor.appearance_id != 48227
            || rail.appearance_id != 12113
            || [floor, rail].iter().any(|facts| {
                facts.pickupable
                    || facts.count != 1
                    || !matches!(facts.definition, MapDefinition::Item(_))
            })
        {
            return false;
        }
        let mut native = Vec::new();
        for x in 33300..=33306 {
            for y in 32052..=32059 {
                let pos = TilePos { x, y, floor: -7 };
                let Some(tile) = base.tile(x, y, -7) else {
                    continue;
                };
                if self.house_tile(pos) {
                    return false;
                }
                let mut entries = Vec::new();
                for (ordinal, (&id, _)) in tile
                    .ids()
                    .iter()
                    .zip(tile.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .enumerate()
                {
                    let Ok(ordinal) = u8::try_from(ordinal) else {
                        return false;
                    };
                    let (Some(key), Some(facts)) = (
                        placement_key(pos, ordinal),
                        self.base_entry(pos, ordinal, id),
                    ) else {
                        return false;
                    };
                    entries.push((key, facts));
                }
                native.push((pos, entries));
            }
        }
        let Ok(mut guard) = self.callbacks.lock() else {
            return false;
        };
        let states = Arc::make_mut(&mut guard);
        if !present {
            let Some(previous) = states.cake_bridge.take() else {
                return true;
            };
            for (pos, tile) in previous.tiles {
                if let Some(entries) = states.movement_items.get_mut(&pos) {
                    entries.retain(|entry| entry.source != view::EntrySource::Added(tile.created));
                    entries.extend(tile.displaced);
                }
                for key in tile.hidden {
                    states.hidden.remove(&key);
                }
                for (key, previous) in tile.previous_passable {
                    if let Some(value) = previous {
                        states.passable.insert(key, value);
                    } else {
                        states.passable.remove(&key);
                    }
                }
            }
            return true;
        }
        if states.cake_bridge.is_some() {
            return true;
        }
        if !states.has_capacity(native.len()) {
            return false;
        }
        let mut projection = BridgeProjection::default();
        for (pos, entries) in native {
            let (created, speed) = if (33301..=33305).contains(&pos.x) {
                (floor, 150)
            } else {
                (rail, 100)
            };
            // The producer leaves a tile alone when its expected bridge Item exists.
            if entries.iter().any(|(key, facts)| {
                !states.hidden.contains(key) && facts.appearance_id == created.appearance_id
            }) || states.movement_items.get(&pos).is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == created.appearance_id)
            }) {
                continue;
            }
            let Some(id) = EntryId::world_change(pos, created.appearance_id) else {
                return false;
            };
            let mut hidden = Vec::new();
            let mut previous_passable = Vec::new();
            for (key, _) in entries {
                if states.hidden.insert(key) {
                    hidden.push(key);
                }
                previous_passable.push((key, states.passable.insert(key, true)));
            }
            let displaced = states.movement_items.remove(&pos).unwrap_or_default();
            states.movement_items.insert(
                pos,
                vec![view::ComposedEntry {
                    source: view::EntrySource::Added(id),
                    facts: created,
                }],
            );
            projection.tiles.insert(
                pos,
                BridgeTile {
                    created: id,
                    hidden,
                    previous_passable,
                    displaced,
                    speed,
                },
            );
        }
        states.cake_bridge = Some(projection);
        true
    }

    pub fn cake_bridge_ground_speed(&self, pos: TilePos) -> Option<u16> {
        self.callbacks
            .lock()
            .ok()?
            .cake_bridge
            .as_ref()?
            .ground_speed(pos)
    }
}
