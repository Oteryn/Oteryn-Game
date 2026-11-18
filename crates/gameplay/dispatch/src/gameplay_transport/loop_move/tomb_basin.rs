//! A scarab coin dropped onto a source coal basin pays for whoever occupies its flame.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_loop_move::{
    LoopMoveReceipt, TOMB_BASINS, TombBasinRule, TombBasinTarget,
};
use crate::durability::npc_travel::PendingArrival;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ChannelRuntimeV1, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::map::boot::BundleMap;

type Commit = Result<Result<LoopMoveReceipt, LoopMoveRefusal>, crate::durability::DurabilityError>;

pub(super) struct Target {
    pos: TilePos,
    key: u64,
    rule: TombBasinRule,
}

pub(super) fn target(map: &BundleMap, to: WorldTilePosition) -> Option<Target> {
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    let pinned = map.facts.pinned();
    if pinned.house_tile(pos) {
        return None;
    }
    let tile = view::compose_tile(&map.overlay, &pinned, pos).ok()??;
    let mut basins = tile.entries.iter().filter_map(|entry| {
        let view::MapEntryKey::Base(key) = entry.source.key() else {
            return None;
        };
        if entry.facts.appearance_id != 2114 {
            return None;
        }
        let rule = TOMB_BASINS.into_iter().find(|rule| {
            map.facts
                .quest_callback_matches(key, Some(rule.uid), None, None)
        })?;
        Some(Target { pos, key, rule })
    });
    let basin = basins.next()?;
    basins.next().is_none().then_some(basin)
}

fn local((x, y, floor): (i32, i32, i16)) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

type Positioned = (
    ExactActorRef,
    MovementPositionSnapshot,
    Option<GameSessionId>,
);

fn occupant(
    runtime: &ChannelRuntimeV1,
    rule: TombBasinRule,
) -> Result<Option<Positioned>, LoopMoveRefusal> {
    let flame = local(rule.flame);
    let actors = runtime
        .positioned_actor_census()
        .map_err(|_| LoopMoveRefusal::Stale)?;
    Ok(actors
        .into_iter()
        .rfind(|(_, position, _)| position.position() == flame))
}

impl Target {
    pub(super) fn emit_poff(&self, map: &BundleMap) {
        map.facts.emit_movement_effect(self.pos, 3);
    }

    pub(super) async fn commit(
        &self,
        admission: &ComposedFreshAdmission<'_, '_, '_>,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: impl ItemLoopContent,
        request: &LoopMoveRequest,
    ) -> Option<Commit> {
        let map = admission.bundle_map()?;
        let catalogue = admission.quest_catalogue.as_deref()?;
        let observed = occupant(&*admission.runtime.lock().await, self.rule);
        let occupant_fence = match observed {
            Ok(Some((_, _, Some(session)))) => admission
                .current_quest_fence(session)
                .await
                .map_err(|()| LoopMoveRefusal::Stale)
                .and_then(|fence| fence.map(Some).ok_or(LoopMoveRefusal::Stale)),
            Ok(_) => Ok(None),
            Err(refusal) => Err(refusal),
        };
        let mut revision_slot = match occupant_fence {
            Ok(Some(fence)) => Some(
                admission
                    .revision_sequencer
                    .acquire(fence.character_id)
                    .await,
            ),
            _ => None,
        };
        let mut runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut step = None;
        let occupant_fence = occupant_fence.and_then(|fence| {
            let Some(fence) = fence else {
                return Ok(None);
            };
            let Ok(Some((actor, before, Some(session)))) = observed else {
                return Err(LoopMoveRefusal::Stale);
            };
            if occupant(&runtime, self.rule)? != observed.ok().flatten()
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, session)
            {
                return Err(LoopMoveRefusal::Stale);
            }
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, admission.owner_now());
            step = Some(
                crate::map::floor::prepare_scripted_quest_teleport_facing(
                    &runtime,
                    admission.movement_cells(),
                    actor,
                    session,
                    before,
                    local(self.rule.destination),
                    blocked,
                    &closed,
                    None,
                    true,
                )
                .map_err(|_| LoopMoveRefusal::Blocked)?,
            );
            Ok(Some(fence))
        });
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend(self.key.to_be_bytes());
        let target = TombBasinTarget {
            uid: self.rule.uid,
            placement,
            revision: map.facts.object_revision(self.key),
            reset_epoch: admission.reset_epoch,
            occupant: occupant_fence,
        };
        let outcome = admission
            .root
            .commit_item_loop_tomb_basin(
                permit,
                admission.character,
                admission.holder,
                fence,
                command,
                &content,
                request.clone(),
                target,
                catalogue,
            )
            .await;
        if let Some(slot) = revision_slot.as_mut() {
            slot.observe_endgame_progression();
        }
        let committed = match outcome {
            Ok(Some(Ok(committed))) => committed,
            Ok(None) => return None,
            Ok(Some(Err(refusal))) => return Some(Ok(Err(refusal))),
            Err(error) => return Some(Err(error)),
        };
        if committed.sacrifice {
            map.facts.emit_movement_effect(self.pos, 16);
        }
        if let Some(arrival) = committed.arrival {
            let Some((fence, proof, (actor, before, Some(session)))) = occupant_fence
                .ok()
                .flatten()
                .zip(step)
                .zip(observed.ok().flatten())
                .map(|((fence, proof), actor)| (fence, proof, actor))
            else {
                return Some(Err(crate::durability::DurabilityError::Unavailable));
            };
            let Ok(after) = runtime.commit_source_step(proof) else {
                return Some(Err(crate::durability::DurabilityError::Unavailable));
            };
            let mut states = states;
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                admission.owner_now().get(),
                Some(before.position()),
                after.position(),
                None,
            );
            let placed = last_position(
                admission,
                &runtime,
                &states,
                actor,
                session,
                after.position(),
            );
            drop(states);
            drop(runtime);
            if !settle(admission, fence, actor, arrival, placed).await {
                return Some(Err(crate::durability::DurabilityError::Unavailable));
            }
            emit_arrival(map, after.position());
        }
        Some(Ok(Ok(committed.receipt)))
    }
}

fn last_position(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    states: &super::super::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    position: MovementLocalPosition,
) -> crate::durability::character_last_position::FinalLastPosition {
    let mut placed = admission.placed_record(runtime, position);
    placed.record.vitals = states
        .saved_vitals(runtime, actor, session)
        .flatten()
        .and_then(|(health, mana)| {
            crate::durability::character_last_position::LastVitals::new(health, mana)
        });
    placed
}

fn emit_arrival(map: &BundleMap, position: MovementLocalPosition) {
    if let Some(tile) = crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }) {
        map.facts.emit_movement_effect(tile, 11);
    }
}

async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    actor: ExactActorRef,
    arrival: PendingArrival,
    placed: crate::durability::character_last_position::FinalLastPosition,
) -> bool {
    if admission
        .root
        .consume_npc_pending_arrival_placed(
            admission.character,
            admission.holder,
            fence,
            arrival.occurrence,
            placed,
        )
        .await
        .is_err()
    {
        return false;
    }
    admission.track_respawned(fence.game_session_id, actor, &placed, fence);
    admission.schedule_quest_refresh(fence.game_session_id);
    true
}

pub(super) async fn reconcile(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    payer: crate::domain::CharacterId,
    command: CommandRef,
) -> bool {
    let pending = admission
        .root
        .read_tomb_basin_arrival(admission.character, payer, command)
        .await;
    let (uid, session, arrival) = match pending {
        Ok(Some(pending)) => pending,
        Ok(None) => return true,
        Err(_) => return false,
    };
    let (Some(rule), Some(map)) = (
        TOMB_BASINS.into_iter().find(|rule| rule.uid == uid),
        admission.bundle_map(),
    ) else {
        return false;
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return false;
    };
    let fence = match admission.current_quest_fence(session).await {
        Ok(Some(fence)) => fence,
        // An unplaced recipient has the durable arrival for its next admission.
        Ok(None) => return true,
        Err(()) => return false,
    };
    let (actor, placed) = {
        let mut runtime = admission.runtime.lock().await;
        let Ok(Some((actor, before))) = runtime.positioned_player_for_session(session) else {
            return true;
        };
        let mut states = admission.states.spell_states.lock().await;
        let to = local(rule.destination);
        if states.is_dead(actor) || states.has_pending_spell_commit(actor, session) {
            return false;
        }
        if before.position() != to {
            if before.position() != local(rule.flame) {
                return false;
            }
            let closed = crate::map::floor::BundleDoors(&map.facts);
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, admission.owner_now());
            let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
                &runtime,
                admission.movement_cells(),
                actor,
                session,
                before,
                to,
                blocked,
                &closed,
                None,
                true,
            ) else {
                return false;
            };
            let Ok(after) = runtime.commit_source_step(proof) else {
                return false;
            };
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                admission.owner_now().get(),
                Some(before.position()),
                after.position(),
                None,
            );
            emit_arrival(map, after.position());
        }
        (
            actor,
            last_position(admission, &runtime, &states, actor, session, to),
        )
    };
    settle(admission, fence, actor, arrival, placed).await
}
