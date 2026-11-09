//! Crystal ff7ede59 svargrond_arena.lua: pillar=1841, teleport=5022.
use super::*;
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};

pub(super) fn pillar(pit: u8) -> Option<TilePos> {
    let at = center(pit)?;
    Some(TilePos {
        x: (at.x - 6).try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: -7,
    })
}

pub(super) fn project(map: &BundleMap, pit: u8, open: bool) -> bool {
    let Some(pos) = pillar(pit) else { return false };
    let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return false;
    };
    let mut entries = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            (map.facts.quest_callback_base_appearance(pos, ordinal, *id) == Some(1841))
                .then_some((ordinal, *id))
        });
    let Some((ordinal, id)) = entries.next() else {
        return false;
    };
    if entries.next().is_some() {
        return false;
    }
    let Some(key) = placement_key(pos, ordinal) else {
        return false;
    };
    let appearance = if open { 5022 } else { 1841 };
    if map
        .facts
        .base_entry(pos, ordinal, id)
        .is_some_and(|facts| facts.appearance_id == appearance)
    {
        return true;
    }
    let Some(mut after) = map.facts.movement_item_facts(appearance) else {
        return false;
    };
    after.bound = true;
    after.pickupable = false;
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        after,
        &[],
        &[(key, after, open)],
        false,
        None,
    )
}


pub(super) fn owns_exit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    at: MovementLocalPosition,
) -> bool {
    owner.states.arena_matches.live.lock().is_ok_and(|matches| {
        matches.get(&session).is_some_and(|live| {
            live.actor == actor && live.state.phase == 2 && live.deadline > Instant::now()
                && pillar(live.state.pit).is_some_and(|p| {
                    at.x == i32::from(p.x) && at.y == i32::from(p.y) && at.floor == -i16::from(p.floor)
                })
        })
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_arena_exit(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: crate::foundation::MovementPositionSnapshot,
    ) -> crate::foundation::MovementPositionSnapshot {
        if !owns_exit(self, actor, session, entered.position()) { return entered; }
        let Some(command) = super::super::quest_callbacks::step_command(actor, session, entered.revision(), self.reset_epoch) else { return entered; };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else { return entered; };
        let caller = {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            let Some(channel) = binding.channel_id() else { return entered; };
            ArenaCaller { actor, supplied: fence.item_fence(), world: binding.world_id(), channel }
        };
        let cause = durable::MatchCause::Step { command, actor: actor.placement_identity(), revision: entered.revision(), epoch: self.reset_epoch };
        let Ok(payload) = wire::encode_match_request(A::Advance) else { return entered; };
        let _ = self.arena_match_native(caller, cause, A::Advance, &payload, Some(entered)).await;
        self.runtime.lock().await.read_actor_position(actor).unwrap_or(entered)
    }
}
//! Crystal ff7ede59 svargrond_arena.lua: pillar=1841, teleport=5022.
use super::*;
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};

pub(super) fn pillar(pit: u8) -> Option<TilePos> {
    let at = center(pit)?;
    Some(TilePos {
        x: (at.x - 6).try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: -7,
    })
}

pub(super) fn project(map: &BundleMap, pit: u8, open: bool) -> bool {
    let Some(pos) = pillar(pit) else { return false };
    let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return false;
    };
    let mut entries = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            (map.facts.quest_callback_base_appearance(pos, ordinal, *id) == Some(1841))
                .then_some((ordinal, *id))
        });
    let Some((ordinal, id)) = entries.next() else {
        return false;
    };
    if entries.next().is_some() {
        return false;
    }
    let Some(key) = placement_key(pos, ordinal) else {
        return false;
    };
    let appearance = if open { 5022 } else { 1841 };
    if map
        .facts
        .base_entry(pos, ordinal, id)
        .is_some_and(|facts| facts.appearance_id == appearance)
    {
        return true;
    }
    let Some(mut after) = map.facts.movement_item_facts(appearance) else {
        return false;
    };
    after.bound = true;
    after.pickupable = false;
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        after,
        &[],
        &[(key, after, open)],
        false,
        None,
    )
}
