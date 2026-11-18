//! M51 source reset before a new fight: Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:207-246.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_decay_retire::ArenaGroundObservation;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::map::overlay::TilePos;
use crate::map::view::{EntrySource, compose_tile};
use oteryn_gameplay_quests::quest::barbarian_arena::{Pit, ResetTopThing};

pub(super) async fn reset_ground(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    permit: &SpellLanePermit,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    state: &MatchState,
) -> Option<()> {
    if state.phase != 1 {
        return Some(());
    }
    let layout = Pit::new(state.pit)?.layout();
    let map = owner.bundle_map()?;
    let native = owner.active_generation?.native_gameplay()?;
    let ground =
        super::super::loop_move::loop_ground(owner, map.overlay.base(), native.item_index())
            .await?;
    let observations = {
        let runtime = owner.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || runtime
                .player_control_facts(actor, fence.game_session_id)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let occupants = runtime.positioned_actor_census().ok()?;
        let blocked: std::collections::BTreeSet<_> = occupants
            .iter()
            .map(|(_, position, _)| {
                let at = position.position();
                [at.x, at.y, -i32::from(at.floor)]
            })
            .collect();
        if layout.occupied_by_other(
            &fence.game_session_id,
            occupants.into_iter().filter_map(|(_, position, session)| {
                let at = position.position();
                session.map(|session| (session, [at.x, at.y, -i32::from(at.floor)]))
            }),
        ) {
            return None;
        }
        let pinned = map.facts.pinned();
        let facts = ground.facts(&pinned);
        let mut observations = Vec::new();
        for at @ [x, y, floor] in layout.reset_cells() {
            if blocked.contains(&at) {
                continue;
            }
            let pos = TilePos {
                x: x.try_into().ok()?,
                y: y.try_into().ok()?,
                floor: floor.try_into().ok()?,
            };
            let Some(top) = compose_tile(ground.overlay(), &facts, pos)
                .ok()?
                .and_then(|tile| tile.entries.last().copied())
            else {
                continue;
            };
            let EntrySource::Ground(item) = top.source else {
                continue;
            };
            let appearance = top.facts.appearance_id;
            let movable = native.item_policy(&format!("oteryn:item.tibia.i{appearance}"), "definition-r1")
                .is_some_and(|policy| {
                    use crate::content::ReferenceItemField::Known;
                    matches!(&policy.record().semantics.physical, Known(physical) if physical.movable == Known(true))
                });
            if layout
                .item_to_reset(
                    at,
                    ResetTopThing::Item {
                        identity: item,
                        appearance,
                        movable,
                    },
                )
                .is_some()
            {
                observations.push(ArenaGroundObservation {
                    item,
                    appearance,
                    spatial_position: ground.pickup_cell(&item)?,
                    stack_ordinal: ground.ground_ordinal(&item)?,
                });
            }
        }
        observations
    };
    let _cache_change = super::super::loop_move::GroundChange(
        (!observations.is_empty()).then_some(&owner.states.loop_ground),
    );
    let retired = owner
        .root
        .retire_arena_pit_ground(
            permit,
            owner.character,
            owner.holder,
            fence,
            state,
            observations,
        )
        .await
        .ok()?;
    for item in retired {
        owner
            .states
            .browse_fields
            .committed_move(super::super::item_view::ItemKey::Instance(item), None);
    }
    Some(())
}

pub(super) fn reset_monsters(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    state: &MatchState,
) -> Option<()> {
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
    let layout = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(state.pit)?.layout();
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
