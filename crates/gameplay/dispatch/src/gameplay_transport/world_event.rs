//! Server-owned scheduled announcement; the player can observe, never activate it.
use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand};
use crate::durability::character_progression::CharacterProgressionError;
use oteryn_protocol_oteryn::world_event::decode_world_event_query;

/// None is an unknown/unavailable commit: disconnect without a terminal result.
pub(super) type Reply = Option<Result<Vec<u8>, ()>>;

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    command: UseCommand,
    payload: &[u8],
) -> Reply {
    if decode_world_event_query(payload).is_err() {
        return Some(Err(()));
    }
    authority.world_event(command).await
}

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Reply {
    let Some(fence) = command.item_fence else {
        return Some(Err(()));
    };
    if fence.game_session_id != command.game_session_id
        || fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id)
    {
        return Some(Err(()));
    }
    match owner
        .root
        .query_world_event(
            owner.character,
            owner.holder,
            fence,
            command.command_id,
            owner.reset_epoch,
        )
        .await
    {
        Ok(bytes) => Some(Ok(bytes)),
        Err(CharacterProgressionError::Unavailable(_)) => None,
        Err(_) => Some(Err(())),
    }
}

/// A native World Board is a static OTBM object. Resolve the pinned placement
/// before observing the World event and send the line through existing chat egress.
pub(super) async fn use_board(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    map: &crate::map::boot::BundleMap,
    target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
) -> Option<super::connection::UseOutcome> {
    use super::connection::UseOutcome;
    use crate::durability::world_event::bank_robbery::BOARD_LINE;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let mut runtime = owner.runtime.lock().await;
    let source = super::world_map::MapViewSource {
        overlay: &map.overlay,
        facts: &map.facts,
        content_generation: runtime.content_pin().client_artifact_digest(),
        reset_epoch: owner.reset_epoch,
    };
    let resolved = super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
    let pos = resolved.pos;
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let (&id, _) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(resolved.ordinal))?;
    let entry = map.facts.base_entry(pos, resolved.ordinal, id)?;
    let MapDefinition::Item(reference) = entry.definition else {
        return None;
    };
    let active = owner.active_generation.filter(|active| {
        active.identity().server_artifact_digest() == runtime.content_pin().server_artifact_digest()
    })?;
    let items = active.native_gameplay()?.item_index()?;
    // Crystal's world_board.lua registers 19236. The Fandom World Board
    // article identifies 19245 as the second orientation of the same object.
    let is_board = ["oteryn:item.tibia.i19236", "oteryn:item.tibia.i19245"]
        .iter()
        .any(|key| {
            items
                .revision(key)
                .and_then(|revision| items.definition_ref("Item", key, revision))
                == Some(reference)
        });
    if !is_board || entry.pickupable {
        return None;
    }
    if owner.states.spell_states.lock().await.is_dead(actor)
        || !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return Some(UseOutcome::rejected());
    }
    let position = runtime
        .borrow_movement_position()
        .read(actor)
        .ok()?
        .position();
    let disposition = if !oteryn_gameplay_items::reach::within_reach(
        (position.x, position.y, i32::from(position.floor)),
        (
            i32::from(pos.x),
            i32::from(pos.y),
            i32::from(owner.runtime_floor(i16::from(pos.floor))),
        ),
    ) {
        UseDisposition::TooFar
    } else if !owner
        .states
        .world_gate
        .lock()
        .is_ok_and(|mut gate| gate.admit(actor, owner.clock_origin.elapsed()))
    {
        UseDisposition::Exhausted
    } else {
        drop(runtime);
        let Some(fence) = command.item_fence.filter(|fence| {
            fence.game_session_id == command.game_session_id
                && fence.runtime_scope
                    == crate::foundation::RuntimeScopeRefV1::channel(
                        owner.world_id,
                        owner.channel_id,
                    )
                && fence.runtime_scope == actor.runtime_scope()
                && fence.scope_ownership_generation == actor.scope_generation()
        }) else {
            return Some(UseOutcome::rejected());
        };
        // Like the Towncryer, read the robbery directly. The Board does not
        // display the separate, byte-limited V1 World Event payload.
        let Ok(announced) = owner
            .root
            .bank_robbery_board_announcement(
                owner.character,
                owner.holder,
                fence,
                owner.reset_epoch,
            )
            .await
        else {
            return Some(UseOutcome::rejected());
        };
        let runtime = owner.runtime.lock().await;
        if owner.states.spell_states.lock().await.is_dead(actor)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || runtime
                .read_actor_position(actor)
                .ok()
                .map(|read| read.position())
                != Some(position)
            || runtime.content_pin().client_artifact_digest() != source.content_generation
        {
            return Some(UseOutcome::rejected());
        }
        if announced {
            owner
                .states
                .chat
                .push_quest_feedback(command.game_session_id, BOARD_LINE);
        }
        UseDisposition::Committed
    };
    Some(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}
