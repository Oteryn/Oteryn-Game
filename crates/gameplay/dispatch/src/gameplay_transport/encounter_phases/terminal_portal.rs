//! Crystal 201189f8, dangerous_depth/creaturescripts_bosses_mission_depths.lua:1-26.
use crate::content::NativeEntryMovementCells;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::view::{MapFacts, placement_key};
use crate::map::{boot::BundleCollisionIndexExt, facts::QuestCallbackProjection, overlay::TilePos};
use oteryn_gameplay_quests::quest::world_callbacks::{
    CorpsePortalCleanup, Position, TerminalPortal,
};

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

/// Crystal 201189f8, the_first_dragon/creaturescripts_death_fallen.lua:1-5,10-13.
/// The movement owner removes the original scene entry. Corpse custody has its own expiry.
pub(super) fn schedule_cleanup(
    cells: &NativeEntryMovementCells,
    position: crate::foundation::MovementLocalPosition,
    cleanup: CorpsePortalCleanup,
    now_us: u64,
) {
    let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
        return;
    };
    let Some(at) = super::super::quest_callbacks::native(position) else {
        return;
    };
    let map = index.map();
    let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
        return;
    };
    if map.facts.house_tile(at) {
        return;
    }
    let Some(due) = now_us.checked_add(u64::from(cleanup.delay_ms) * 1_000) else {
        return;
    };
    let source = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            let revision = map.facts.object_revision(key);
            let entry = map.facts.base_entry(at, ordinal, *id)?;
            (entry.appearance_id == cleanup.appearance
                && !entry.pickupable
                && !map.facts.base_hidden(key))
            .then_some((key, revision, entry))
        });
    if let Some((key, revision, entry)) = source {
        map.facts.apply_quest_callback(
            key,
            revision,
            entry,
            &[],
            &[],
            false,
            Some(&QuestCallbackProjection::TimedRemoval { due, entry }),
        );
    }
}
