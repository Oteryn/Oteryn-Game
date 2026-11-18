//! Crystal 201189f8, dangerous_depth/creaturescripts_bosses_mission_depths.lua:1-26.
use crate::content::NativeEntryMovementCells;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::view::{MapFacts, placement_key};
use crate::map::{boot::BundleCollisionIndexExt, facts::QuestCallbackProjection, overlay::TilePos};
use oteryn_gameplay_quests::quest::world_callbacks::{Position, TerminalPortal};

#[derive(Debug)]
pub(super) struct Portal {
    plan: TerminalPortal,
    due_us: u64,
}

fn tile(position: Position) -> TilePos {
    TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

impl Portal {
    pub(super) fn open(plan: TerminalPortal, now_us: u64) -> Option<Self> {
        let due_us = now_us.checked_add(u64::from(plan.duration_ms) * 1_000)?;
        Some(Self { plan, due_us })
    }

    pub(super) fn project(&self, cells: &NativeEntryMovementCells, now_us: u64) -> bool {
        if now_us >= self.due_us {
            return true;
        }
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return true;
        };
        let map = index.map();
        let at = tile(self.plan.position);
        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return true;
        };
        let mut sources = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(at, ordinal)?;
                let restore = map.facts.base_entry(at, ordinal, *id)?;
                (restore.appearance_id == self.plan.original_appearance
                    && map.facts.quest_callback_native_teleport(
                        key,
                        self.plan.original_appearance,
                        tile(self.plan.return_destination),
                    ))
                .then_some((key, restore))
            });
        let Some((key, restore)) = sources.next() else {
            return true;
        };
        if sources.next().is_some() {
            return true;
        }
        let Some(after) = map
            .facts
            .quest_callback_replacement(key, self.plan.open_appearance)
        else {
            return true;
        };
        map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            after,
            &[],
            &[],
            false,
            Some(&QuestCallbackProjection::TimedEntrance {
                due: self.due_us,
                restore,
                position: at,
                destination: tile(self.plan.destination),
            }),
        )
    }
}
