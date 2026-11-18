//! Crystal 201189f8, dangerous_depth/creaturescripts_bosses_mission_depths.lua:1-26.
use crate::content::NativeEntryMovementCells;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::view::{MapFacts, placement_key};
use crate::map::{
    boot::BundleCollisionIndexExt,
    facts::{QuestCallbackProjection, QuestPortalTravel},
    overlay::TilePos,
};
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
                travel: QuestPortalTravel::Native {
                    effect: self.plan.traversal_effect,
                },
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
    schedule_tile_removals(cells, position, cleanup.delay_ms, now_us, 1, |entry| {
        entry.appearance_id == cleanup.appearance && !entry.pickupable
    });
}

/// Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:1-9,12-22.
/// The callback tests the item's type, so placement action and unique IDs do not qualify it.
pub(super) fn schedule_fixed_item_cleanup(
    cells: &NativeEntryMovementCells,
    position: crate::foundation::MovementLocalPosition,
    delay_ms: u32,
    now_us: u64,
) {
    use crate::map::spell_tiles::source_item_type_movable;
    use oteryn_world_bundle::bundle::TerrainKind;

    schedule_tile_removals(cells, position, delay_ms, now_us, usize::MAX, |entry| {
        entry.terrain_kind != Some(TerrainKind::Ground)
            && !entry.pickupable
            && source_item_type_movable(entry.appearance_id) == Some(false)
    });
}

fn schedule_tile_removals(
    cells: &NativeEntryMovementCells,
    position: crate::foundation::MovementLocalPosition,
    delay_ms: u32,
    now_us: u64,
    limit: usize,
    matches: impl Fn(crate::map::view::EntryFacts) -> bool,
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
    let Some(due) = now_us.checked_add(u64::from(delay_ms) * 1_000) else {
        return;
    };
    let source = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            let revision = map.facts.object_revision(key);
            let entry = map.facts.base_entry(at, ordinal, *id)?;
            (matches(entry) && !map.facts.base_hidden(key)).then_some((key, revision, entry))
        })
        .take(limit);
    for (key, revision, entry) in source {
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
