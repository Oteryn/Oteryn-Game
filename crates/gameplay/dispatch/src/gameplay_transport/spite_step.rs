//! Crystal soul_war_mechanics.lua:434-480: the original committed Player STEP
//! selects one current searing fire or the original33792 Item-id callbacks.
//! These use the existing atomic Quest/Ground owner. Weeping's Item-only
//! callback uses the physical Item ledger and the same movement reservation.
//! SQL uncertainty retains that original translation and its exact Items.
mod creature;
mod megalomania;
mod weeping;
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{
    QuestTransitionOutcome,
    soul_war_source::{
        SoulWarSourceAction,
        step::{PreparedSpiteStep, SoulWarSourceStep, SourceStepKind},
    },
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot,
    SourceActorReservation, SourceActorReservationProof,
};
use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(super) struct SpiteStepState {
    pub(super) creatures: Arc<crate::movement::monster_map::SourceCreatureStepQueue>,
    pending: Vec<PendingStep>,
    completed: Vec<(ExactActorRef, GameSessionId, u64)>,
    recovery: Vec<(ExactActorRef, GameSessionId, u64)>,
}

#[derive(Debug, Clone)]
struct PendingStep {
    movement: PreparedSpiteStep,
    reservation: SourceActorReservation,
    selected: Option<(SoulWarSourceStep, CurrentCharacterGameplayFence)>,
    completed_items: Vec<[u8; 16]>,
    fire_finished: bool,
    quest_finished: bool,
    weeping: Option<weeping::PendingWeeping>,
    megalomania: Option<megalomania::PendingMegalomania>,
}

impl SpiteStepState {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.recovery.iter().map(|(_, _, due)| *due).min()
    }
}

/// Called synchronously at the physical movement commit, before field cleanup
/// awaits. The paced caller also covers the synchronous entry-room fallback;
/// an already retained translation is never replaced by that second call.
pub(super) fn stage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
) -> Result<(), ()> {
    stage_with_items(owner, runtime, states, actor, session, movement, None)
}

pub(super) fn stage_from_current_read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
    read: &crate::durability::spell_item_transaction::StandingTileRead,
) -> Result<(), ()> {
    stage_with_items(owner, runtime, states, actor, session, movement, Some(read))
}

fn stage_with_items(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
    read: Option<&crate::durability::spell_item_transaction::StandingTileRead>,
) -> Result<(), ()> {
    let Some(native) = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
    else {
        return Ok(());
    };
    let Some(room) = owner.effective_rooms().gate_field_step_ingress() else {
        return Ok(());
    };
    let proof = if let Some(read) = read {
        PreparedSpiteStep::from_committed_movement_with_items(
            runtime,
            native,
            room,
            actor,
            session,
            movement,
            owner.reset_epoch,
            read,
        )
    } else {
        PreparedSpiteStep::from_committed_movement(
            runtime,
            native,
            room,
            actor,
            session,
            movement,
            owner.reset_epoch,
        )
    };
    let Some(proof) = proof else {
        return Ok(());
    };
    let state = &mut states.spite_steps;
    state
        .completed
        .retain(|(actor, session, _)| runtime.player_control_facts(*actor, *session).is_ok());
    if state
        .completed
        .iter()
        .any(|(old_actor, old_session, revision)| {
            *old_actor == actor && *old_session == session && *revision >= movement.revision()
        })
        || state.pending.iter().any(|entry| {
            entry.movement.actor() == actor
                && entry.movement.session() == session
                && entry.movement.movement() == movement
        })
    {
        return Ok(());
    }
    let mut reservation = runtime
        .prepare_source_actor_reservation(&proof)
        .map_err(|_| ())?;
    runtime
        .reserve_source_actors(&mut reservation, &proof)
        .map_err(|_| ())?;
    // No await separates the physical reservation from its retained source.
    state.pending.push(PendingStep {
        movement: proof,
        reservation,
        selected: None,
        completed_items: Vec::new(),
        fire_finished: false,
        quest_finished: false,
        weeping: None,
        megalomania: None,
    });
    states.owner_wake.notify_one();
    Ok(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[cfg(test)]
    pub(super) async fn retained_spite_movement_for_test(
        &self,
    ) -> Option<MovementPositionSnapshot> {
        self.states
            .spell_states
            .lock()
            .await
            .spite_steps
            .pending
            .first()
            .map(|pending| pending.movement.movement())
    }

    #[cfg(test)]
    pub(super) async fn retained_spite_step_for_test(
        &self,
    ) -> Option<(SoulWarSourceStep, CurrentCharacterGameplayFence)> {
        self.states
            .spell_states
            .lock()
            .await
            .spite_steps
            .pending
            .first()?
            .selected
            .clone()
    }

    pub(super) async fn resolve_spite_steps(&self, permit: &SpellLanePermit) -> bool {
        // A preceding field COMMIT must install or prove rollback first. The
        // retained source reservation already protects this exact translation.
        if permit.has_unresolved() {
            return false;
        }
        // Every entry is one actual admitted actor's already committed movement;
        // a pass snapshots that finite queue, without a new retry budget.
        let count = self
            .states
            .spell_states
            .lock()
            .await
            .spite_steps
            .pending
            .iter()
            .map(|pending| pending.movement.callback_count().saturating_add(1))
            .sum::<usize>();
        for _ in 0..count {
            if self.resolve_one_spite_step(permit).await.is_err() {
                return false;
            }
        }
        self.states
            .spell_states
            .lock()
            .await
            .spite_steps
            .pending
            .is_empty()
    }

    async fn resolve_one_spite_step(&self, permit: &SpellLanePermit) -> Result<(), ()> {
        let pending = self
            .states
            .spell_states
            .lock()
            .await
            .spite_steps
            .pending
            .first()
            .cloned();
        let Some(mut pending) = pending else {
            return Ok(());
        };
        if pending.quest_finished {
            let selected = pending
                .movement
                .source_ground_steps()
                .iter()
                .find(|input| !pending.completed_items.contains(&input.item_instance_id()))
                .map(|input| input.kind());
            return match selected {
                Some(crate::durability::creature_source_items::SourceGroundStepKind::Weeping) => {
                    self.resolve_one_weeping_step(permit, pending).await
                }
                Some(_) => self.resolve_one_megalomania_step(permit, pending).await,
                None => {
                    let mut runtime = self.runtime.lock().await;
                    let mut states = self.states.spell_states.lock().await;
                    finish(&mut runtime, &mut states, &pending)
                }
            };
        }
        if pending.selected.is_none() {
            let fence = self
                .current_quest_fence(pending.movement.session())
                .await?
                .ok_or(())?;
            if !pending.movement.matches_fence(&fence.item_fence()) {
                return Err(());
            }
            {
                let runtime = self.runtime.lock().await;
                runtime
                    .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                    .map_err(|_| ())?;
            }
            let read = self
                .root
                .read_standing_player_tile(
                    permit,
                    self.character,
                    self.holder,
                    fence.item_fence(),
                    pending.movement.content_digest(),
                    pending.movement.target().clone(),
                )
                .await
                .map_err(|_| ())?;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            runtime
                .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                .map_err(|_| ())?;
            let native = self
                .active_generation
                .and_then(|generation| generation.native_gameplay())
                .ok_or(())?;
            let room = self.effective_rooms().gate_field_step_ingress().ok_or(())?;
            let selected = pending
                .movement
                .qualify_current_next(
                    &runtime,
                    native,
                    room,
                    &read,
                    &pending.completed_items,
                    pending.fire_finished,
                )
                .ok_or(())?;
            let Some(selected) = selected else {
                if pending.movement.source_ground_steps().is_empty() {
                    finish(&mut runtime, &mut states, &pending)?;
                } else {
                    pending.quest_finished = true;
                    let entry = states
                        .spite_steps
                        .pending
                        .iter_mut()
                        .find(|entry| {
                            entry.movement.event_identity() == pending.movement.event_identity()
                        })
                        .ok_or(())?;
                    *entry = pending;
                }
                return Ok(());
            };
            pending.selected = Some((selected, fence));
            let entry = states
                .spite_steps
                .pending
                .iter_mut()
                .find(|entry| entry.movement.event_identity() == pending.movement.event_identity())
                .ok_or(())?;
            *entry = pending.clone();
        }
        let (input, fence) = pending.selected.as_ref().ok_or(())?;
        let catalogue = self.quest_catalogue.as_ref().ok_or(())?;
        let action = SoulWarSourceAction::Step(input.clone());
        let request = action.request(fence).map_err(|_| ())?;
        if input.kind() == SourceStepKind::Remorse {
            // A known second Remorse must have a representable counter/icon
            // successor before its Item can commit. The physical reservation
            // keeps this same Player current across the detached SQL write.
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            runtime
                .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                .map_err(|_| ())?;
            states
                .get(
                    &runtime,
                    pending.movement.actor(),
                    pending.movement.session(),
                )
                .ok_or(())?
                .stage_soul_war_torment_counter(0)
                .ok_or(())?;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        // Retain the original fence after selection. The sole writer reconciles
        // its immutable receipt BEFORE any fresh authority or Ground lookup.
        let result = slot
            .commit_soul_war_source_action(
                self.root,
                self.character,
                self.holder,
                *fence,
                action,
                Arc::clone(catalogue),
                Some((permit, input.context())),
            )
            .await;
        drop(slot);
        let committed = match &result {
            Ok(
                QuestTransitionOutcome::Committed(receipt)
                | QuestTransitionOutcome::AlreadyCommitted(receipt),
            ) => {
                let [change] = receipt.changes.as_slice() else {
                    return Err(());
                };
                if receipt.character_id != fence.character_id
                    || receipt.cause != request.cause
                    || receipt.transition_key != request.transition_key
                    || receipt.quest_key != rule::QUEST
                    || receipt.completes
                    || receipt.experience.is_some()
                {
                    return Err(());
                }
                Some(match input.kind() {
                    SourceStepKind::SpiteFire
                        if change.track == rule::SPITE_FIRE
                            && change.after >= change.before.checked_add(56).ok_or(())? =>
                    {
                        AppliedStep::Fire(input.recovery_remaining_micros(change.after).ok_or(())?)
                    }
                    SourceStepKind::Remorse
                        if change.track == rule::REMORSE
                            && matches!(change.before, 0 | 1)
                            && change.after == 1 - change.before =>
                    {
                        AppliedStep::Remorse {
                            reset: change.before == 1,
                        }
                    }
                    _ => return Err(()),
                })
            }
            Err(CharacterProgressionError::Unavailable(_)) => return Err(()),
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed)
            | Err(_) => None,
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        runtime
            .validate_source_actor_reservation(&pending.reservation, &pending.movement)
            .map_err(|_| ())?;
        if let Some(AppliedStep::Remorse { reset: true }) = committed {
            let current = states
                .get(
                    &runtime,
                    pending.movement.actor(),
                    pending.movement.session(),
                )
                .ok_or(())?;
            let next = current.stage_soul_war_torment_counter(0).ok_or(())?;
            if next.revision() != current.revision()
                && !states.commit(
                    &runtime,
                    pending.movement.actor(),
                    pending.movement.session(),
                    next,
                )
            {
                return Err(());
            }
        }
        if committed.is_some() {
            self.states.loop_ground.invalidate();
        }
        if let Some(AppliedStep::Fire(recovery_micros)) = committed {
            self.states.server_messages.send_current(pending.movement.session(),ServerMessage::Advance("The soul fire was stomped out in time! Your soul will now have to recover before you can do this again.".into()));
            states.spite_steps.recovery.push((
                pending.movement.actor(),
                pending.movement.session(),
                self.owner_now().get().saturating_add(recovery_micros),
            ));
            states.owner_wake.notify_one();
        } else if committed.is_none() && input.cooldown_remaining() > 0 {
            self.states.server_messages.send_current(
                pending.movement.session(),
                ServerMessage::Advance(format!(
                    "His soul won't need to recover again! You need wait {} seconds.",
                    input.cooldown_remaining()
                )),
            );
        }
        if let Some(AppliedStep::Remorse { reset: true }) = committed
            && let Some(room) = self.effective_rooms().gate_field_step_ingress()
            && let Some(map) = super::soul_war_scene::source_map(room)
            && let Some(position) =
                super::soul_war_scene::wire_position(pending.movement.movement().position())
        {
            let captured_name = self
                .states
                .chat
                .speaker_name(pending.movement.session())
                .and_then(|name| {
                    oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&name).ok()
                });
            map.facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: pending.movement.actor(),
                    position,
                    talk_type: 36,
                    text: "The remorse calms your dread!".into(),
                    captured_name,
                });
            map.facts.emit_movement_effect(position, 50);
        }
        pending.completed_items.push(input.item_instance_id());
        pending.fire_finished |= input.kind() == SourceStepKind::SpiteFire;
        pending.selected = None;
        let entry = states
            .spite_steps
            .pending
            .iter_mut()
            .find(|entry| entry.movement.event_identity() == pending.movement.event_identity())
            .ok_or(())?;
        *entry = pending;
        Ok(())
    }

    pub(super) async fn drain_spite_recovery_notices(&self) {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err() {
            return;
        }
        let now = self.owner_now().get();
        states.spite_steps.recovery.retain(|(actor, session, due)| {
            if runtime.player_control_facts(*actor, *session).is_err() {
                return false;
            }
            if *due > now {
                return true;
            }
            self.states.server_messages.send_current(
                *session,
                ServerMessage::Advance("Your soul has recovered!".into()),
            );
            false
        });
    }
}

#[derive(Clone, Copy)]
enum AppliedStep {
    Fire(u64),
    Remorse { reset: bool },
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static SPITE_STEP_CAPTURED_TEST_BARRIER: Arc<tokio::sync::Notify>;
}

#[cfg(test)]
pub(super) async fn pause_captured_step_for_test() {
    if let Ok(entered) = SPITE_STEP_CAPTURED_TEST_BARRIER.try_with(Arc::clone) {
        entered.notify_one();
        std::future::pending::<()>().await;
    }
}

fn finish(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    pending: &PendingStep,
) -> Result<(), ()> {
    let index = states
        .spite_steps
        .pending
        .iter()
        .position(|entry| entry.movement.event_identity() == pending.movement.event_identity())
        .ok_or(())?;
    runtime
        .release_source_actors_after_observed_outcome(
            pending.reservation.clone(),
            &pending.movement,
        )
        .map_err(|_| ())?;
    let actor = pending.movement.actor();
    let session = pending.movement.session();
    states
        .spite_steps
        .completed
        .retain(|(old_actor, old_session, _)| *old_actor != actor || *old_session != session);
    states
        .spite_steps
        .completed
        .push((actor, session, pending.movement.movement().revision()));
    states.spite_steps.pending.remove(index);
    Ok(())
}
