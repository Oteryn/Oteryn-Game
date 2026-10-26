//! Reconcile the confirmed travel commit, place once, then consume its obligation.
use super::npc_travel::{FeeOutcome, TravelCapture, TravelStep};
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::npc_travel::ConsumeReplay;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn newhaven_chase_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(StepOutcome, Option<Duration>)> {
        if self.states.newhaven_travel.held(session) {
            return None;
        }
        self.chase_step_now(actor, session).await
    }

    pub(super) async fn consume_admitted_newhaven_arrival(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: Option<&last_position::AdmissionPosition>,
    ) -> bool {
        let Some(last_position::AdmissionPosition::Arrival(pending, vitals, rest)) = decided else {
            if !self.consume_admitted_respawn(session, actor, decided).await {
                return false;
            }
            return self.complete_tutorial_islet_admission(actor, session).await;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let placed = {
            let runtime = self.runtime.lock().await;
            if runtime.player_control_facts(actor, session).is_err()
                || runtime.owner_fence().is_err()
            {
                return false;
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return false;
            };
            let mut placed = self.placed_record(&runtime, position.position());
            placed.record.vitals = *vitals;
            placed.record.rest = *rest;
            placed
        };
        if self
            .root
            .consume_npc_pending_arrival_placed(
                self.character,
                self.holder,
                fence,
                pending.occurrence,
                placed,
            )
            .await
            .is_err()
        {
            return false;
        }
        self.track_respawned(session, actor, &placed, fence);
        true
    }

    /// None keeps the invocation held. A typed refusal is a known abort; a database
    /// error never releases the hold or repeats a placement already recorded in memory.
    pub(super) async fn reconcile_newhaven_travel(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<bool> {
        let fence = self.current_quest_fence(session).await.ok()??;
        let (frozen, capture, route, committed, mut placed) = {
            let holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get(&session)?;
            if held.actor != actor || held.capture.session != fence.item_fence() {
                return None;
            }
            if !held.slot.is_held() {
                return Some(false);
            }
            (
                held.frozen.clone(),
                held.capture.clone(),
                held.route.clone(),
                held.committed,
                held.placed,
            )
        };
        if !committed {
            let result = match route {
                newhaven_travel::Departure::Tutorial(route) => self
                    .root
                    .commit_newhaven_travel(
                        self.character,
                        self.holder,
                        fence,
                        frozen.command,
                        frozen.arrival,
                        route,
                    )
                    .await
                    .map(Ok),
                newhaven_travel::Departure::Paid(request) => {
                    self.root
                        .commit_npc_travel(
                            self.character,
                            self.holder,
                            fence,
                            *request,
                            &self.premium_coordinator,
                        )
                        .await
                }
            };
            let mut holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get_mut(&session)?;
            match result {
                Err(CharacterProgressionError::Unavailable(_)) => return None,
                Ok(Err(refusal)) => {
                    held.refusal = Some(refusal);
                    held.slot
                        .fee_outcome(frozen.arrival.occurrence, FeeOutcome::Aborted)
                        .ok()?;
                    return Some(false);
                }
                Err(_) => {
                    held.slot
                        .fee_outcome(frozen.arrival.occurrence, FeeOutcome::Aborted)
                        .ok()?;
                    return Some(false);
                }
                Ok(Ok(None)) => {
                    holds.remove(&session);
                    return Some(true);
                }
                Ok(Ok(Some(_))) => {
                    held.slot
                        .fee_outcome(frozen.arrival.occurrence, FeeOutcome::Committed)
                        .ok()?;
                    held.committed = true;
                }
            }
        }
        if placed.is_none() {
            let current = self.current_quest_fence(session).await.ok()??;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let position = runtime.read_actor_position(actor).ok()?;
            let independently_current = TravelCapture {
                session: current.item_fence(),
                content: position.context(),
                position_revision: position.revision(),
            };
            let mut holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get_mut(&session)?;
            if held.slot.placement_decision(
                &independently_current,
                runtime.owner_fence().ok()?,
                !states.is_dead(actor),
                states.is_dead(actor),
            ) != TravelStep::PlacementPending
                || capture.session != current.item_fence()
                || states.has_pending_spell_commit(actor, session)
            {
                return None;
            }
            let destination = frozen.arrival.destination;
            let destination = crate::foundation::MovementLocalPosition {
                x: destination.x,
                y: destination.y,
                floor: destination.floor,
            };
            // NPC travel is a teleport too. Stage its existing cooldown map before physical
            // placement, and publish it once alongside that placement, never on reconciliation.
            let cooldowns = if destination != position.position() {
                use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
                let until = self.owner_now().checked_add(STAIRHOP_MICROS).ok()?;
                states.get(&runtime, actor, session).map(|state| {
                    let mut next = state.cooldowns.clone();
                    next.groups.insert(STAIRHOP_COOLDOWN_KEY.into(), until);
                    next
                })
            } else {
                None
            };
            let doors = std::collections::BTreeSet::new();
            let digest = runtime.content_pin().server_artifact_digest();
            let snapshot = runtime
                .place_admitted_respawn(actor, session, Some(destination), |cell| {
                    respawn_cell_admissible(
                        self.movement_cells.index(),
                        self.movement_cells.scope(),
                        self.world_id,
                        digest,
                        &doors,
                        cell,
                    ) && !self.bundle_map().is_some_and(|map| {
                        crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
                            x: cell.x,
                            y: cell.y,
                            z: i32::from(cell.floor),
                        })
                        .is_none_or(|tile| map.facts.doors().blocks(tile))
                    })
                })
                .ok()?;
            if let Some(cooldowns) = cooldowns
                && let Some(state) = states.get_mut(&runtime, actor, session)
            {
                state.cooldowns = cooldowns;
            }
            let mut record = self.placed_record(&runtime, snapshot.position());
            record.record.vitals = states
                .saved_vitals(&runtime, actor, session)
                .flatten()
                .and_then(|(health, mana)| {
                    crate::durability::character_last_position::LastVitals::new(health, mana)
                });
            held.slot
                .placement_recorded(frozen.arrival.occurrence, std::time::Instant::now())
                .ok()?;
            held.placed = Some(record);
            placed = Some(record);
            self.states.chat.close_out_of_range(&runtime, session);
        }
        let current = self.current_quest_fence(session).await.ok()??;
        if current.item_fence() != capture.session {
            return None;
        }
        let placed = placed?;
        let consumed = self
            .root
            .consume_npc_pending_arrival_placed(
                self.character,
                self.holder,
                current,
                frozen.arrival.occurrence,
                placed,
            )
            .await
            .ok()?;
        if consumed == crate::durability::npc_travel::ArrivalConsume::Consumed {
            self.track_respawned(session, actor, &placed, current);
        }
        let mut holds = self.states.newhaven_travel.holds.lock().ok()?;
        let held = holds.get_mut(&session)?;
        held.slot
            .consume_outcome(frozen.arrival.occurrence, ConsumeReplay::Complete)
            .ok()?;
        holds.remove(&session);
        Some(true)
    }
}
