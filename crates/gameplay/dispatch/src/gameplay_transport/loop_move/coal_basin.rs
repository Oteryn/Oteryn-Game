//! Native AddItem and recoverable same-Channel landing for the Ancient Tombs.
//! Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:30-55.
use std::sync::OnceLock;

use oteryn_gameplay_quests::quest::world_callbacks::{
    self, Position,
    coal_basins::{self, CoalBasin},
};

use super::*;
use crate::{
    durability::{item_loop_move::CoalBasinTarget, npc_travel::ArrivalConsume},
    foundation::MovementLocalPosition,
    map::view::{MapFacts, placement_key},
};

fn bindings<'a>(admission: &'a ComposedFreshAdmission<'_, '_, '_>) -> &'a [CoalBasin] {
    #[cfg(test)]
    if let Some(bindings) = admission.states.quest_callbacks.coal_basins_fixture.get() {
        return bindings;
    }
    let _ = admission;
    static BINDINGS: OnceLock<Vec<CoalBasin>> = OnceLock::new();
    BINDINGS.get_or_init(|| coal_basins::load(world_callbacks::EMBEDDED).unwrap_or_default())
}

fn local(position: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}

pub(super) async fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    to: WorldTilePosition,
) -> Result<Option<CoalBasinTarget>, LoopMoveResult> {
    let (Ok(x), Ok(y), Ok(floor)) = (
        u16::try_from(to.x),
        u16::try_from(to.y),
        i8::try_from(to.floor),
    ) else {
        return Ok(None);
    };
    let to = TilePos { x, y, floor };
    let Some(binding) = bindings(admission).iter().find(|binding| {
        (
            binding.basin.position.x,
            binding.basin.position.y,
            binding.basin.position.floor,
        ) == (to.x, to.y, to.floor)
    }) else {
        return Ok(None);
    };
    let refused = LoopMoveResult::Refused(LoopMoveRefusal::Rejected);
    let map = admission.bundle_map().ok_or(refused)?;
    let tile = map
        .overlay
        .base()
        .tile(to.x, to.y, to.floor)
        .ok_or(refused)?;
    let ordinal = binding.basin.ordinal;
    let key = placement_key(to, ordinal).ok_or(refused)?;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or(refused)?;
    if map.facts.house_tile(to)
        || map.facts.base_hidden(key)
        || !map
            .facts
            .quest_callback_native_object(key, id, binding.basin.appearance)
    {
        return Err(refused);
    }
    let occupant = {
        let runtime = admission.runtime.lock().await;
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| LoopMoveResult::Unknown)?;
        census
            .into_iter()
            .rev()
            .find(|(_, snapshot, _)| snapshot.position() == local(binding.flame))
            .and_then(|(actor, _, session)| {
                let session = session?;
                let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                Some((actor, session, lease))
            })
    };
    let recipient = if let Some((actor, session, lease)) = occupant {
        let catalogue = admission.quest_catalogue.clone().ok_or(refused)?;
        let fence = admission
            .current_quest_fence(session)
            .await
            .map_err(|_| LoopMoveResult::Unknown)?
            .ok_or(refused)?
            .item_fence();
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || runtime.bound_attacker_lease(actor, session).ok() != Some(lease)
            || runtime
                .read_actor_position(actor)
                .ok()
                .map(|snapshot| snapshot.position())
                != Some(local(binding.flame))
            || states.is_dead(actor)
        {
            return Err(refused);
        }
        Some((fence, catalogue))
    } else {
        None
    };
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(Some(CoalBasinTarget {
        binding: binding.clone(),
        placement,
        reset_epoch: i64::try_from(admission.reset_epoch).map_err(|_| refused)?,
        recipient,
    }))
}

pub(super) async fn reconcile(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    depositor: crate::domain::CharacterId,
    command: CommandRef,
) -> bool {
    if bindings(admission).is_empty() {
        return true;
    }
    let Ok(landing) = admission
        .root
        .read_coal_basin_landing(admission.character, depositor, command)
        .await
    else {
        return false;
    };
    let Some(landing) = landing else {
        return true;
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return false;
    };
    let transfer = {
        let runtime = admission.runtime.lock().await;
        let Ok(census) = runtime.positioned_melee_census() else {
            return false;
        };
        census.into_iter().find_map(|(actor, _, session)| {
            let session = session?;
            let lease = runtime.bound_attacker_lease(actor, session).ok()?;
            (lease.character_id().as_bytes() == &landing.recipient).then_some((actor, session))
        })
    };
    let Some((actor, session)) = transfer else {
        // The normal first-entry owner consumes the retained landing on reconnect.
        return true;
    };
    let Ok(Some(mut fence)) = admission.current_quest_fence(session).await else {
        return false;
    };
    if fence.character_id.as_bytes() != &landing.recipient
        || fence.runtime_scope.world_id() != landing.arrival.world_id
    {
        return false;
    }
    let mut slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    // The item transaction may have advanced this independent Character root.
    slot.observe_home_town();
    let Ok(revision) = slot.cursor(admission.root, admission.character).await else {
        return false;
    };
    fence.expected_character_revision = revision;
    let Some(map) = admission.bundle_map() else {
        return false;
    };
    let destination = landing.arrival.destination;
    let destination = MovementLocalPosition {
        x: destination.x,
        y: destination.y,
        floor: destination.floor,
    };
    let placed = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor) || runtime.bound_attacker_lease(actor, session).is_err() {
            return true;
        }
        let Ok(current) = runtime.read_actor_position(actor) else {
            return false;
        };
        if current.position() != destination {
            let closed = crate::map::floor::BundleDoors(&map.facts);
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, admission.owner_now());
            let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
                &runtime,
                admission.movement_cells(),
                actor,
                session,
                current,
                destination,
                blocked,
                &closed,
                current.facing(),
                false,
            ) else {
                return false;
            };
            if runtime.commit_source_step(proof).is_err() {
                return false;
            }
            if let Some(tile) =
                crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
                    x: destination.x,
                    y: destination.y,
                    z: i32::from(destination.floor),
                })
            {
                map.facts.emit_movement_effect(tile, 11);
            }
            admission.states.chat.close_out_of_range(&runtime, session);
        }
        let mut placed = admission.placed_record(&runtime, destination);
        placed.record.vitals = states
            .saved_vitals(&runtime, actor, session)
            .flatten()
            .and_then(|(hp, mana)| {
                crate::durability::character_last_position::LastVitals::new(hp, mana)
            });
        placed
    };
    let outcome = admission
        .root
        .consume_npc_pending_arrival_placed(
            admission.character,
            admission.holder,
            fence,
            landing.arrival.occurrence,
            placed,
        )
        .await;
    slot.observe_home_town();
    drop(slot);
    if matches!(outcome, Ok(ArrivalConsume::Consumed)) {
        admission.track_respawned(session, actor, &placed, fence);
    }
    drop(_permit);
    admission.refresh_quest_session(session, false).await;
    outcome.is_ok()
}
