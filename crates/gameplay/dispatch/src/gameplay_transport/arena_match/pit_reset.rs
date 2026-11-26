//! M51 source reset before a new fight: Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:207-246.
use super::*;
use crate::map::overlay::TilePos;
use crate::map::view::{EntrySource, compose_tile};
use oteryn_gameplay_quests::quest::barbarian_arena::{Pit, PitLayout};
use oteryn_protocol_oteryn::world_map::MapDefinition;

pub(super) fn reset_monsters(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    state: &MatchState,
) -> Option<()> {
    let layout = reset_layout(runtime, actor, session, state)?;
    // The native view separates Monsters from NPCs/Players. Hold this guard
    // through selection and removal; an administrative removal grants no kill.
    let monsters = runtime.visible_entities().creatures;
    let selected = layout.monsters_to_reset(monsters.into_iter().map(|monster| {
        let at = monster.position;
        (monster.actor, [at.x, at.y, -i32::from(at.floor)])
    }))?;
    for target in selected {
        // Removing a parent also removes its bounded summons. A planned child
        // already gone through that exact native owner is an idempotent absence.
        if runtime.contains_live_creature(target) {
            runtime.remove_arena_opponent(target).ok()?;
        }
    }
    Some(())
}

fn reset_layout(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    state: &MatchState,
) -> Option<PitLayout> {
    runtime.owner_fence().ok()?;
    if actor.channel_id().is_none()
        || state.phase != 1
        || state.session != *session.as_bytes()
        || state.scope_generation != runtime.binding().scope_generation().get()
        || runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
    {
        return None;
    }
    let layout = Pit::new(state.pit)?.layout();
    let players = runtime.positioned_actor_census().ok()?;
    if layout.occupied_by_other(
        &session,
        players.into_iter().filter_map(|(_, at, player)| {
            let at = at.position();
            player.map(|player| (player, [at.x, at.y, -i32::from(at.floor)]))
        }),
    ) {
        return None;
    }
    Some(layout)
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static ARENA_ITEM_RESET_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

pub(super) async fn reset_items(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    state: &MatchState,
) -> Option<durable::ArenaPitResetOutcome> {
    let Some(map) = owner.bundle_map() else {
        return Some(durable::ArenaPitResetOutcome::Applied);
    };
    let pit = Pit::new(state.pit)?;
    let native = owner.active_generation?.native_gameplay()?;
    let index = {
        let runtime = owner.runtime.lock().await;
        permit.check_scope(runtime.binding().runtime_scope()).ok()?;
        super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?
    };
    let ground =
        super::super::loop_move::loop_ground(owner, map.overlay.base(), Some(index)).await?;
    let items = {
        let runtime = owner.runtime.lock().await;
        let layout = reset_layout(&runtime, actor, session, state)?;
        if owner.states.spell_states.try_lock().ok()?.is_dead(actor)
            || !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || !owner
                .states
                .loop_ground
                .current_for(&runtime, map.overlay.base())
                .is_some_and(|current| std::sync::Arc::ptr_eq(&current, &ground))
        {
            return None;
        }
        let entities = runtime.visible_entities();
        let occupied: std::collections::BTreeSet<_> = entities
            .players
            .into_iter()
            .chain(entities.creatures)
            .chain(entities.npcs)
            .map(|entity| {
                [
                    entity.position.x,
                    entity.position.y,
                    -i32::from(entity.position.floor),
                ]
            })
            .collect();
        let facts = ground.facts(&map.facts);
        let mut items = Vec::new();
        for cell @ [x, y, floor] in layout.reset_cells() {
            if occupied.contains(&cell) {
                continue;
            }
            let tile = TilePos {
                x: x.try_into().ok()?,
                y: y.try_into().ok()?,
                floor: floor.try_into().ok()?,
            };
            let Ok(Some(stack)) = compose_tile(ground.overlay(), &facts, tile) else {
                continue;
            };
            let Some(top) = stack.entries.last() else {
                continue;
            };
            let (EntrySource::Ground(identity), MapDefinition::Item(definition)) =
                (top.source, top.facts.definition)
            else {
                continue;
            };
            let Some(item) = durable::ArenaPitResetItem::observe(
                native,
                pit,
                identity,
                &ground.pickup_cell(&identity)?,
                ground.ground_ordinal(&identity)?,
                definition,
            ) else {
                continue;
            };
            items.push(item);
        }
        items
    };
    // A concurrent LOOK can cache the old Ground while retirement is in flight.
    // Invalidate again on completion or cancellation, including a lost acknowledgement.
    let cache = (!items.is_empty()).then_some(&owner.states.loop_ground);
    let _ground_change = super::super::loop_move::GroundChange(cache);
    if !items.is_empty() {
        owner.states.loop_ground.invalidate();
    }
    #[cfg(test)]
    if let Ok(barrier) = ARENA_ITEM_RESET_BARRIER.try_with(std::sync::Arc::clone) {
        barrier.0.notify_one();
        barrier.1.notified().await;
    }
    owner
        .root
        .retire_arena_pit_items(permit, owner.character, owner.holder, fence, state, items)
        .await
        .ok()
}
