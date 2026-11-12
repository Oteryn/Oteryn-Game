//! SRVGAP-C1: the existing map-object USE route owns Dawnport's vocation doors.
//! Crystal 201189f8 `data-global/scripts/actions/dawnport/vocation_door.lua:1-45`.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
};
use crate::durability::character_last_position::FinalLastPosition;
use crate::durability::dawnport_departure::DepartureOutcome;
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_items::starter_grant::starting_loadout;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone)]
struct Pending {
    actor: ExactActorRef,
    command: UseCommand,
    unique: u16,
    digest: [u8; 32],
    placed: FinalLastPosition,
}

#[derive(Default)]
pub(super) struct Departures {
    records: Mutex<HashMap<GameSessionId, Pending>>,
    #[cfg(test)]
    pub(super) interrupt_after_commit: std::sync::atomic::AtomicBool,
}
impl Departures {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut records) = self.records.lock() {
            records.remove(&session);
        }
    }
    pub(super) fn held(&self, session: GameSessionId) -> bool {
        self.records
            .lock()
            .map_or(true, |records| records.contains_key(&session))
    }
}
fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_dawnport_departure(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        if self
            .states
            .dawnport_departures
            .held(command.game_session_id)
            && !self
                .resume_dawnport_departure(actor, command.game_session_id)
                .await
        {
            return Some(UseOutcome::rejected());
        }
        let _permit = self.spell_lane_permit().await?;
        // Resolve only the immutable authored identity before the historical receipt check.
        let (unique, digest) = {
            let runtime = self.runtime.lock().await;
            let source = super::world_map::MapViewSource {
                overlay: &map.overlay,
                facts: &map.facts,
                content_generation: runtime.content_pin().client_artifact_digest(),
                reset_epoch: 0,
            };
            let resolved =
                super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
            let unique = (22001..=22004).find(|unique| {
                map.facts
                    .quest_callback_matches(resolved.placement_key, Some(*unique), None, None)
            })?;
            (unique, runtime.content_pin().server_artifact_digest())
        };
        let Some(fence) = command.item_fence else {
            return Some(UseOutcome::rejected());
        };
        let Ok(id) = CommandId::new(command.command_id) else {
            return Some(UseOutcome::rejected());
        };
        match self
            .root
            .replay_dawnport_departure(
                self.character,
                self.holder,
                fence,
                CommandRef::new(command.game_session_id, id),
                unique,
                digest,
            )
            .await
        {
            Ok(true) => return Some(outcome(UseDisposition::Committed)),
            Ok(false) => {}
            Err(_) => return Some(UseOutcome::rejected()),
        }
        if !self
            .checkpoint_timed_equipment(actor, command.game_session_id, true)
            .await
        {
            return Some(UseOutcome::rejected());
        }
        let pending = {
            let runtime = self.runtime.lock().await;
            let source = super::world_map::MapViewSource {
                overlay: &map.overlay,
                facts: &map.facts,
                content_generation: runtime.content_pin().client_artifact_digest(),
                reset_epoch: 0,
            };
            let resolved =
                super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
            let unique = (22001..=22004).find(|unique| {
                map.facts
                    .quest_callback_matches(resolved.placement_key, Some(*unique), None, None)
            })?;
            let (_, (x, y, floor)) = starting_loadout::departure(unique)?;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return Some(UseOutcome::rejected());
            };
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, None)
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Some(UseOutcome::rejected());
            }
            let expected = runtime.read_actor_position(actor).ok()?;
            let from = expected.position();
            if !oteryn_gameplay_items::reach::within_reach(
                (from.x, from.y, i32::from(from.floor)),
                (
                    i32::from(resolved.pos.x),
                    i32::from(resolved.pos.y),
                    -i32::from(resolved.pos.floor),
                ),
            ) {
                return Some(outcome(UseDisposition::TooFar));
            }
            let destination = MovementLocalPosition { x, y, floor };
            let cell = crate::content::LogicalCell {
                x,
                y,
                z: i32::from(floor),
            };
            let tile = crate::map::overlay::TilePos {
                x: x.try_into().ok()?,
                y: y.try_into().ok()?,
                floor: -i8::try_from(floor).ok()?,
            };
            if self
                .movement_cells()
                .index()
                .lookup(self.movement_cells().scope(), cell)
                != Ok(crate::content::CollisionClass::Walkable)
                || map.facts.doors().blocks(tile)
                || map.overlay.has_added_item(tile)
                || (from != destination && !runtime.native_summon_cell_free(destination))
            {
                return Some(UseOutcome::rejected());
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return Some(outcome(UseDisposition::Exhausted));
            }
            let mut placed = self.placed_record(&runtime, destination);
            if let Some(state) = states.get(&runtime, actor, command.game_session_id) {
                placed.record.rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
                placed.record.vitals = crate::durability::character_last_position::LastVitals::new(
                    state.vitals().health,
                    state.vitals().mana,
                );
            }
            Pending {
                actor,
                command,
                unique,
                digest: runtime.content_pin().server_artifact_digest(),
                placed,
            }
        };
        let Ok((current, _)) = super::FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(command.game_session_id)
            .await
        else {
            return Some(UseOutcome::rejected());
        };
        if command.item_fence.is_none()
            || super::item_fence_of(current) != command.item_fence
            || CommandId::new(command.command_id).is_err()
        {
            return Some(UseOutcome::rejected());
        }
        self.states
            .dawnport_departures
            .records
            .lock()
            .ok()?
            .insert(command.game_session_id, pending.clone());
        Some(self.finish_dawnport_departure(pending).await)
    }

    /// Movement/logout finish an interrupted commit before they can overwrite its saved landing.
    pub(super) async fn resume_dawnport_departure(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let pending = match self.states.dawnport_departures.records.lock() {
            Ok(records) => records.get(&session).cloned(),
            Err(_) => return false,
        };
        let Some(pending) = pending else {
            return true;
        };
        if pending.actor != actor {
            // A new actor restores the durable landing through ordinary first entry.
            if let Ok(mut records) = self.states.dawnport_departures.records.lock() {
                records.remove(&session);
            }
            return true;
        }
        if self.states.spell_states.lock().await.is_dead(actor) {
            // Death's pending respawn supersedes the landing; its own transaction owns placement.
            if let Ok(mut records) = self.states.dawnport_departures.records.lock() {
                records.remove(&session);
            }
            return true;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        self.finish_dawnport_departure(pending).await.disposition == UseDisposition::Committed
    }

    async fn finish_dawnport_departure(&self, pending: Pending) -> UseOutcome {
        let session = pending.command.game_session_id;
        let Some(supplied) = pending.command.item_fence else {
            return UseOutcome::rejected();
        };
        let Ok((current, _)) = super::FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
        else {
            return UseOutcome::rejected();
        };
        let Some(fresh) = super::item_fence_of(current) else {
            return UseOutcome::rejected();
        };
        // oteryn-choice: authenticated same-session recovery may advance only the connection;
        // the frozen departure keeps its Character lease, scope, command and content identity.
        let mut refreshed = supplied;
        refreshed.connection_generation = fresh.connection_generation;
        if refreshed != fresh {
            return UseOutcome::rejected();
        }
        {
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(pending.actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || runtime.content_pin().server_artifact_digest() != pending.digest
            {
                return UseOutcome::rejected();
            }
        }
        let Ok(id) = CommandId::new(pending.command.command_id) else {
            return UseOutcome::rejected();
        };
        let result = self
            .root
            .depart_dawnport(
                self.character,
                self.holder,
                fresh,
                CommandRef::new(session, id),
                pending.unique,
                pending.digest,
                pending.placed,
            )
            .await;
        let disposition = match result {
            Ok(DepartureOutcome::Committed) => {
                #[cfg(test)]
                if self
                    .states
                    .dawnport_departures
                    .interrupt_after_commit
                    .swap(false, std::sync::atomic::Ordering::SeqCst)
                {
                    return UseOutcome::rejected();
                }
                // Use the same timed-item and live-effect publication as an ordinary equipment move.
                if !self
                    .checkpoint_timed_equipment(pending.actor, session, true)
                    .await
                {
                    return UseOutcome::rejected();
                }
                self.states
                    .attack
                    .lock()
                    .await
                    .forget_weapon(pending.actor, session);
                let Ok(effects) = self.read_admission_effects(session).await else {
                    return UseOutcome::rejected();
                };
                let mut runtime = self.runtime.lock().await;
                let Ok(states) = self.states.spell_states.try_lock() else {
                    return UseOutcome::rejected();
                };
                if !runtime
                    .player_control_facts(pending.actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                    || states.is_dead(pending.actor)
                    || states.get(&runtime, pending.actor, session).is_none()
                    || runtime.content_pin().server_artifact_digest() != pending.digest
                {
                    return UseOutcome::rejected();
                }
                drop(states);
                if self
                    .install_admission_suppressions(&runtime, pending.actor, session, effects)
                    .await
                    .is_none()
                {
                    return UseOutcome::rejected();
                }
                let cell = pending.placed.record.cell;
                let destination = MovementLocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                };
                // oteryn-choice: reuse the existing fenced administrative relocation primitive;
                // it compares the exact actor/session/position and has no arena-specific effects.
                let Ok(expected) = runtime.read_actor_position(pending.actor) else {
                    return UseOutcome::rejected();
                };
                if expected.position() != destination
                    && (!runtime.native_summon_cell_free(destination)
                        || runtime
                            .relocate_arena_player(pending.actor, session, expected, destination)
                            .is_err())
                {
                    return UseOutcome::rejected();
                }
                UseDisposition::Committed
            }
            Ok(DepartureOutcome::AlreadyDeparted) => UseDisposition::NothingToUse,
            Ok(DepartureOutcome::NotEligible) => UseDisposition::Rejected,
            Err(_) => return UseOutcome::rejected(), // Keep the PREPARE across an unknown commit.
        };
        if let Ok(mut records) = self.states.dawnport_departures.records.lock() {
            records.remove(&session);
        }
        outcome(disposition)
    }
}
