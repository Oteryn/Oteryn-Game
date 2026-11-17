//! Crystal 9f5a72c6 data/scripts/actions/items/ladder_up.lua:5-18,21:
//! sewer435 USE descends one floor through the existing movement owner.
use super::*;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::UseOutcome;
use crate::gameplay_transport::world_map::{MapHandleTarget, MapViewSource};
use crate::map::view::{EntrySource, MapFacts, compose_tile};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn sewer(
    map: &crate::map::boot::BundleMap,
    generation: [u8; 32],
    epoch: u64,
    object: Option<&WorldObjectTarget>,
    item: Option<ItemKey>,
) -> Option<TilePos> {
    let facts = map.facts.pinned();
    let source = MapViewSource {
        overlay: &map.overlay,
        facts: &facts,
        content_generation: generation,
        reset_epoch: epoch,
    };
    let target = if let Some(object) = object {
        let target = crate::gameplay_transport::world_map::resolve_map_target(
            &source,
            object,
            |_| None::<()>,
        )
        .ok()?;
        MapHandleTarget::Base {
            pos: target.pos,
            ordinal: target.ordinal,
            placement_key: target.placement_key,
        }
    } else {
        crate::gameplay_transport::world_map::resolve_map_handle(&source, item?).ok()?
    };
    let pos = match target {
        MapHandleTarget::Base { pos, .. } | MapHandleTarget::Added { pos, .. } => pos,
        MapHandleTarget::Ground { .. } => return None,
    };
    if facts.house_tile(pos) {
        return None;
    }
    compose_tile(&map.overlay, &facts, pos)
        .ok()??
        .entries
        .iter()
        .any(|entry| {
            entry.facts.appearance_id == 435
                && match (target, entry.source) {
                    (
                        MapHandleTarget::Base {
                            placement_key: wanted,
                            ..
                        },
                        EntrySource::Base { placement_key, .. },
                    ) => wanted == placement_key,
                    (MapHandleTarget::Added { id, .. }, EntrySource::Added(actual)) => {
                        id == actual.get()
                    }
                    _ => false,
                }
        })
        .then_some(pos)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_bundle_dropdown(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        object: Option<&WorldObjectTarget>,
        item: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        let map = self.bundle_map()?;
        let generation = self.active_generation?.identity().client_artifact_digest();
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            // Only current server-resolved sewer entries claim this route. Re-resolve
            // after the awaited policy reads, under the movement commit lock.
            {
                let _runtime = self.runtime.lock().await;
                sewer(map, generation, self.reset_epoch, object, item)?;
            }
            let disposition = async {
                let Some(permit) = self.spell_lane_permit().await else {
                    return UseDisposition::Rejected;
                };
                if !self.refresh_pvp_locks(actor, command.game_session_id).await {
                    return UseDisposition::Rejected;
                }
                let optional_pvp = self
                    .special_movement_policy(&permit, actor, command.game_session_id)
                    .await;
                let mut runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if states.is_dead(actor)
                    || !states.spell_idle(&runtime, actor, command.game_session_id)
                    || !runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                    || crate::gameplay_transport::item_ref_admission::pinned_index(
                        &runtime,
                        self.active_generation,
                    )
                    .is_none()
                    || map.overlay.runtime_scope() != runtime.binding().runtime_scope()
                    || !self
                        .states
                        .player_trades
                        .physical(command.game_session_id, None)
                {
                    return UseDisposition::Rejected;
                }
                let Some(pos) = sewer(map, generation, self.reset_epoch, object, item) else {
                    return UseDisposition::StaleState;
                };
                let Ok(expected) = runtime.read_actor_position(actor) else {
                    return UseDisposition::Rejected;
                };
                let from = expected.position();
                if !oteryn_gameplay_items::reach::within_reach(
                    (from.x, from.y, i32::from(from.floor)),
                    (i32::from(pos.x), i32::from(pos.y), -i32::from(pos.floor)),
                ) {
                    return UseDisposition::TooFar;
                }
                let Some(floor) = pos
                    .floor
                    .checked_neg()
                    .and_then(|floor| floor.checked_add(1))
                    .filter(|floor| *floor <= 15)
                else {
                    return UseDisposition::NotPossible;
                };
                if !self
                    .states
                    .world_gate
                    .lock()
                    .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
                {
                    return UseDisposition::Exhausted;
                }
                let now = self.owner_now();
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let blocked = crate::gameplay_transport::pvp_live::pz_blocked(
                    &states,
                    actor,
                    command.game_session_id,
                    now,
                );
                let proof = match crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    self.movement_cells(),
                    actor,
                    command.game_session_id,
                    expected,
                    MovementLocalPosition {
                        x: i32::from(pos.x),
                        y: i32::from(pos.y),
                        floor: i16::from(floor),
                    },
                    blocked,
                    &closed,
                ) {
                    Ok(proof) => proof,
                    Err(_) => return UseDisposition::NotPossible,
                };
                use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
                let Ok(until) = now.checked_add(STAIRHOP_MICROS) else {
                    return UseDisposition::Rejected;
                };
                let Ok(snapshot) = runtime.commit_source_step(proof) else {
                    return UseDisposition::Rejected;
                };
                if let Some(state) = states.get_mut(&runtime, actor, command.game_session_id) {
                    state
                        .cooldowns
                        .groups
                        .insert(STAIRHOP_COOLDOWN_KEY.into(), until);
                }
                states.stage_special_movement_step(
                    &runtime,
                    map,
                    actor,
                    command.game_session_id,
                    now.get(),
                    Some(from),
                    snapshot.position(),
                    optional_pvp,
                );
                use crate::gameplay_transport::paced_movement::underwater_drown::underwater;
                states.stage_underwater_step(
                    &runtime,
                    actor,
                    command.game_session_id,
                    now.get(),
                    underwater(map, from),
                    underwater(map, snapshot.position()),
                );
                self.states
                    .chat
                    .close_out_of_range(&runtime, command.game_session_id);
                UseDisposition::Committed
            }
            .await;
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        })
        .await;
        outcome.unwrap_or_else(|_| Some(UseOutcome::rejected()))
    }
}
