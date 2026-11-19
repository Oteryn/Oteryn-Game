//! Crystal 201189f dreamers_challenge_quest/actions_tic_tac_teleport.lua:15-55.
use super::*;
use crate::map::{
    facts::{QuestCallbackProjection, QuestSceneMutation},
    view::{EntryFacts, compose_tile},
};
use oteryn_gameplay_quests::quest::world_callbacks::BoardPortal;

type Ground = super::super::loop_move::LoopGroundOverlay;

pub(super) fn base_slot(
    map: &BundleMap,
    at: TilePos,
    accepted: &[u16],
) -> Result<Option<(u64, EntryFacts)>, UseDisposition> {
    if map.facts.house_tile(at) {
        return Err(UseDisposition::Rejected);
    }
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or(UseDisposition::Rejected)?;
    let mut found = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(index, (id, _))| {
            let ordinal = u8::try_from(index).ok()?;
            let key = placement_key(at, ordinal)?;
            let facts = map.facts.base_entry(at, ordinal, *id)?;
            (!facts.pickupable && accepted.contains(&facts.appearance_id)).then_some((key, facts))
        });
    let source = found.next();
    if found.next().is_some() {
        return Err(UseDisposition::Rejected);
    }
    Ok(source)
}

pub(super) fn prepare(
    map: &BundleMap,
    ground: &Ground,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    source: u64,
    board: &BoardPortal,
    lever_appearance: u16,
) -> Result<QuestCallbackProjection, UseDisposition> {
    let mut mutations = Vec::new();
    let can_restore = |position: Position| {
        let at = local(position);
        runtime
            .read_actor_position(actor)
            .is_ok_and(|current| current.position() != at)
            && runtime
                .position_occupied_by_other(actor, at)
                .is_ok_and(|occupied| !occupied)
            && runtime
                .reserved_into_by_own_pending(actor, at)
                .is_ok_and(|reserved| !reserved)
    };
    for position in &board.pillars {
        let at = tile(*position);
        let Some((key, facts)) = base_slot(map, at, &[board.pillar])? else {
            // Crystal actions_tic_tac_teleport.lua:17-24 creates a pillar
            // independently on each tile that lacks one.
            if let Some(entry) = map.facts.quest_scene_solid_at(source, at, board.pillar) {
                mutations.push(QuestSceneMutation::RemoveCreatedSolid {
                    position: at,
                    entry,
                });
            } else {
                if !can_restore(*position) {
                    return Err(UseDisposition::Occupied);
                }
                let facts = map
                    .facts
                    .movement_item_facts(board.pillar)
                    .filter(|facts| !facts.pickupable)
                    .ok_or(UseDisposition::Rejected)?;
                mutations.push(QuestSceneMutation::Create {
                    position: at,
                    facts,
                    passable: false,
                });
            }
            continue;
        };
        let revision = map.facts.object_revision(key);
        mutations.push(if map.facts.base_hidden(key) {
            if !can_restore(*position) {
                return Err(UseDisposition::Occupied);
            }
            QuestSceneMutation::Restore {
                key,
                revision,
                facts,
                passable: false,
            }
        } else {
            QuestSceneMutation::Remove { key, revision }
        });
    }
    let pinned = map.facts.pinned();
    let facts = ground.facts(&pinned);
    let solved = board.tokens.iter().all(|token| {
        let position = tile(token.position);
        !map.facts.house_tile(position)
            && compose_tile(ground.overlay(), &facts, position)
                .ok()
                .flatten()
                .is_some_and(|stack| {
                    stack
                        .entries
                        .iter()
                        .any(|entry| entry.facts.appearance_id == token.appearance)
                })
    });
    // Crystal actions_tic_tac_teleport.lua:39-55 changes the portal only
    // when the corresponding item is present. The lever and pillars toggle anyway.
    if solved
        && let Some((key, current)) =
            base_slot(map, tile(board.portal), &[board.sealed, board.opened])?
        && !map.facts.base_hidden(key)
    {
        let opened = lever_appearance == board.open_on;
        let desired = if opened { board.opened } else { board.sealed };
        if desired != current.appearance_id {
            let facts = map
                .facts
                .quest_callback_replacement(key, desired)
                .filter(|entry| !entry.pickupable)
                .ok_or(UseDisposition::Rejected)?;
            mutations.push(QuestSceneMutation::Replace {
                key,
                revision: map.facts.object_revision(key),
                facts,
                passable: true,
            });
        }
    }
    Ok(QuestCallbackProjection::Scene {
        mutations,
        remove_self: false,
    })
}
