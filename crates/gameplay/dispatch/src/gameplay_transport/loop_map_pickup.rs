//! MAP-PICKUP-1: command 9 joins the existing exact eligibility, map MINT and item loop owners.
//! The spell permit spans both transactions. A MINT receipt without a move receipt resumes
//! that same item; a different command against its hidden origin is stale without a write.

use super::*;
use crate::durability::map_item_mint::{
    MapItemMintError, MapItemMintOutcome, MapItemMintRequest, plan_map_item_mint,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::map::overlay::pickup::{self, MintResolution};

pub(super) async fn move_map_item(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: LoopMove,
    permit: &SpellLanePermit,
) -> LoopMoveResult {
    let refused = LoopMoveResult::Refused;
    let stale = refused(LoopMoveRefusal::Stale);
    let destination = match request.target {
        LoopTarget::MainBackpack => LoopDestination::MainBackpack,
        LoopTarget::Container(ItemKey::Instance(id)) => LoopDestination::Container(id),
        _ => return refused(LoopMoveRefusal::NotSupported),
    };
    let Some(fence) = command.item_fence else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return refused(LoopMoveRefusal::Rejected);
    };
    // The caller already holds the lane permit: runtime, then spell state, then no guards
    // across durable IO (SPELL-LOCK-2 §1.2), exactly as the instance move.
    let (native, index, at) = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.spell_states.lock().await;
        let native = native_for_move(
            &runtime,
            admission.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        );
        let index = pinned_index(&runtime, admission.active_generation);
        drop(states);
        let at = super::super::bundle_map_position(&mut runtime, admission.movement_cells, actor);
        (native, index, at)
    };
    let Some(native) = native else {
        return refused(LoopMoveRefusal::Rejected);
    };
    let (Some(index), Some(at)) = (index, at) else {
        return refused(LoopMoveRefusal::NotSupported);
    };
    let ItemKey::MapBase {
        bundle_digest,
        placement_key,
        reset_epoch,
    } = request.item
    else {
        return refused(LoopMoveRefusal::NotSupported);
    };
    let base = at.map.overlay.base();
    // Only boot's durable active epoch may name this physical map generation.
    if bundle_digest != base.digest() || reset_epoch != admission.reset_epoch {
        return stale;
    }
    let Some((pos, ordinal)) = view::placement_of(placement_key) else {
        return stale;
    };
    let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
        return stale;
    };
    let Some(id) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
    else {
        return stale;
    };
    let Ok(entry) = at.map.facts.pickup_entry(pos, ordinal, id) else {
        return stale;
    };
    let Some(EntryFacts {
        definition: MapDefinition::Item(reference),
        ..
    }) = at.map.facts.base_entry(pos, ordinal, id)
    else {
        return stale;
    };
    let Some(definition) = crate::gameplay_transport::item_look::typed_definition(index, reference)
    else {
        return stale;
    };
    let Ok(item) = generation_item_facts(native, &definition) else {
        return stale;
    };
    if plan_map_item_mint(&item, entry.quantity, false).is_err() {
        return stale;
    }
    let Some(ground) = loop_ground(admission, base, Some(index)).await else {
        return LoopMoveResult::Unknown;
    };
    let mint = MapItemMintRequest {
        command: CommandRef::new(command.game_session_id, command_id),
        base_bundle_digest: bundle_digest,
        placement_key,
        reset_epoch,
        item,
        quantity: entry.quantity,
        content_revision: sha256_revision(native.source_digest()),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
    };
    let hidden = ground
        .overlay()
        .tile(pos)
        .is_some_and(|tile| tile.is_hidden(ordinal));
    // A crash-retry is the sole exception to the hidden-origin gate. This probe writes nothing.
    if hidden {
        match admission
            .root
            .read_map_item_mint_receipt(admission.character, &fence, &mint)
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => {
                // A prior unknown attempt may have reserved but not committed its MINT.
                // Only that pending command may reconcile it; other commands remain stale.
                let Ok(state) = admission.loop_ground.state.lock() else {
                    return LoopMoveResult::Unknown;
                };
                if state.pending_origins.get(&placement_key) != Some(&mint.command) {
                    return stale;
                }
            }
            Err(error) => return mint_error(error),
        }
    }
    let player = match (
        u16::try_from(at.at.x),
        u16::try_from(at.at.y),
        i8::try_from(at.at.floor),
    ) {
        (Ok(x), Ok(y), Ok(floor)) => TilePos { x, y, floor },
        _ => return LoopMoveResult::TooFar,
    };
    if !pickup::within_reach(player, pos) {
        return LoopMoveResult::TooFar;
    }
    match admission
        .root
        .preflight_map_item_loop_pickup(
            admission.character,
            admission.holder,
            &fence,
            &NativeItemWeights(native),
            &mint,
            &destination,
        )
        .await
    {
        Ok(Ok(())) => {}
        Ok(Err(reason)) => return refused(reason),
        Err(_) => return LoopMoveResult::Unknown,
    }
    // Use the existing overlay budget admission against every currently hidden origin.
    let mut overlay = ChannelOverlay::new(base.clone(), admission.world_id, admission.channel_id);
    let taken: Vec<_> = ground
        .overlay()
        .tiles()
        .flat_map(|(pos, tile)| {
            (0..64)
                .filter(move |ordinal| tile.is_hidden(*ordinal))
                .filter_map(move |ordinal| view::placement_key(pos, ordinal))
        })
        .collect();
    if pickup::rehide_taken_origins(&mut overlay, &taken).is_err() {
        return LoopMoveResult::Unknown;
    }
    if !hidden && pickup::hide_origin_at_freeze(&mut overlay, placement_key).is_err() {
        return refused(LoopMoveRefusal::NoRoom);
    }
    pending_origin(&admission.loop_ground, placement_key, Some(mint.command));
    let mut candidate = match admission
        .root
        .freeze_map_item_mint(admission.character, admission.holder, fence, mint)
        .await
    {
        Ok(candidate) => candidate,
        Err(error @ MapItemMintError::Unavailable(_)) => return mint_error(error),
        Err(error) => {
            pending_origin(&admission.loop_ground, placement_key, None);
            return mint_error(error);
        }
    };
    let committed = if hidden {
        admission
            .root
            .reconcile_map_item_mint(admission.character, &mut candidate)
            .await
    } else {
        match admission
            .root
            .commit_map_item_mint(
                permit,
                admission.character,
                admission.holder,
                fence,
                &mut candidate,
            )
            .await
        {
            Ok(
                MapItemMintOutcome::Committed(minted)
                | MapItemMintOutcome::AlreadyCommitted(minted),
            ) => Ok(Some(minted)),
            // An uncertain commit keeps the origin hidden until reconciliation proves its state.
            Err(MapItemMintError::Unavailable(_)) => {
                admission
                    .root
                    .reconcile_map_item_mint(admission.character, &mut candidate)
                    .await
            }
            Err(
                error @ (MapItemMintError::InvalidInput
                | MapItemMintError::AuthorityRejected
                | MapItemMintError::Refused(_)
                | MapItemMintError::ConflictingCause
                | MapItemMintError::ConflictingCandidate),
            ) => {
                // A semantic refusal proves this attempt did not MINT. A durable receipt
                // belonging to another command is still re-hidden by the invalidated reader.
                let _ = pickup::settle_origin(
                    &mut overlay,
                    placement_key,
                    MintResolution::ProvenNotCommitted,
                );
                pending_origin(&admission.loop_ground, placement_key, None);
                return mint_error(error);
            }
            Err(error @ MapItemMintError::CapacityExceeded) => Err(error),
        }
    };
    let minted = match committed {
        Ok(Some(minted)) => {
            let _ = pickup::settle_origin(
                &mut overlay,
                placement_key,
                MintResolution::Committed(&minted),
            );
            pending_origin(&admission.loop_ground, placement_key, None);
            minted
        }
        Ok(None) => {
            let _ = pickup::settle_origin(
                &mut overlay,
                placement_key,
                MintResolution::ProvenNotCommitted,
            );
            pending_origin(&admission.loop_ground, placement_key, None);
            return LoopMoveResult::Unknown;
        }
        Err(error) => {
            let _ = pickup::settle_origin(&mut overlay, placement_key, MintResolution::Unknown);
            return mint_error(error);
        }
    };
    // The invalidated read proves this exact instance is still on its own tile, even on retry.
    let Some(ground) = loop_ground(admission, base, Some(index)).await else {
        return LoopMoveResult::Unknown;
    };
    if ground
        .ground_entry(&minted.item_instance_id)
        .map(|(at, _)| at)
        != Some(pos)
    {
        return stale;
    }
    let result = admission
        .root
        .commit_item_loop_move(
            permit,
            admission.character,
            admission.holder,
            &fence,
            candidate.request().command,
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: minted.item_instance_id,
                count: None,
                pickup_cell: Some(ground_cell(
                    i32::from(pos.x),
                    i32::from(pos.y),
                    i16::from(pos.floor),
                )),
                destination,
            },
        )
        .await;
    admission.loop_ground.invalidate();
    admission
        .attack
        .lock()
        .await
        .forget_weapon(actor, command.game_session_id);
    match result {
        Ok(Ok(_)) => LoopMoveResult::Committed,
        Ok(Err(reason)) => refused(reason),
        Err(_) => LoopMoveResult::Unknown,
    }
}

fn pending_origin(cache: &LoopGroundCache, key: u64, pending: Option<CommandRef>) {
    if let Ok(mut state) = cache.state.lock() {
        if let Some(command) = pending {
            state.pending_origins.insert(key, command);
        } else {
            state.pending_origins.remove(&key);
        }
        state.epoch = state.epoch.wrapping_add(1);
        state.current = None;
    }
}

fn mint_error(error: MapItemMintError) -> LoopMoveResult {
    match error {
        MapItemMintError::InvalidInput
        | MapItemMintError::ConflictingCause
        | MapItemMintError::ConflictingCandidate
        | MapItemMintError::Refused(_) => LoopMoveResult::Refused(LoopMoveRefusal::Stale),
        MapItemMintError::AuthorityRejected => LoopMoveResult::Refused(LoopMoveRefusal::Rejected),
        MapItemMintError::CapacityExceeded | MapItemMintError::Unavailable(_) => {
            LoopMoveResult::Unknown
        }
    }
}
