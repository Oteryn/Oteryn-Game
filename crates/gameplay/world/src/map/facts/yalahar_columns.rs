//! Crystal items.xml 24509–24548: the fixed room's four paired ten-second decays.
use super::*;

const PERIOD_US: u64 = 10_000_000;

impl BundleFacts {
    pub(super) fn arm_yalahar_columns(&self, states: &mut CallbackStates, now: u64) {
        if states.initialized_yalahar_columns {
            return;
        }
        states.initialized_yalahar_columns = true;
        let Some(due) = now.checked_add(PERIOD_US) else {
            return;
        };
        for x in [32867, 32869, 32871, 32873] {
            let position = TilePos {
                x,
                y: 31253,
                floor: -11,
            };
            if self.house_tile(position) {
                continue;
            }
            let (Some(first), Some(last)) = (
                placement_key(position, 0),
                placement_key(position, MAX_BASE_ORDINAL),
            ) else {
                continue;
            };
            let start = self.placements.partition_point(|(key, _)| *key < first);
            let end = self.placements.partition_point(|(key, _)| *key <= last);
            let mut columns = self.placements[start..end]
                .iter()
                .filter_map(|(key, placement)| {
                    let palette = self.palette.get(placement.palette as usize)?;
                    let facts = palette.facts;
                    (!facts.pickupable && (9133..=9140).contains(&facts.appearance_id))
                        .then_some((*key, placement, palette))
                });
            let Some((key, placement, original)) = columns.next() else {
                continue;
            };
            if columns.next().is_some()
                || states.key_reserved(key)
                || states.hidden.contains(&key)
                || states.changed.contains_key(&key)
                || !states.has_capacity(2)
            {
                continue;
            }
            let current = EntryFacts {
                bound: placement.bound,
                count: u32::from(placement.count),
                sub_type: u32::from(placement.sub_type),
                ..original.facts
            };
            let alternate = if current.appearance_id <= 9136 {
                current.appearance_id + 4
            } else {
                current.appearance_id - 4
            };
            let Some(mut replacement) = self.quest_callback_replacement(key, alternate) else {
                continue;
            };
            if replacement.pickupable || replacement.terrain_kind != current.terrain_kind {
                continue;
            }
            replacement.bound = current.bound;
            replacement.count = current.count;
            replacement.sub_type = current.sub_type;
            states.changed.insert(key, (0, current));
            states.movement_decays.insert(
                (due, key),
                MovementDecay {
                    revision: 0,
                    restore: replacement,
                    repeat_us: Some(PERIOD_US),
                    overlay: None,
                },
            );
        }
    }
}
