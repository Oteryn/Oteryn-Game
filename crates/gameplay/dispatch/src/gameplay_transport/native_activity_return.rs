//! Live return half of ActivityHandoffOwner: reserve, retain, commit and rebind.
//! The retained source room supplies its exit; no client chooses a Channel/cell.
use super::activity_runtime::{RegisteredRuntime, ScopeKey};
use super::actor_spell::PreparedActivitySpellStates;
use super::attack::PreparedActivityAttackStates;
use super::connection::{
    AdmittedSession, ConnectionIdentifiers, ControllerBinding, FirstEntryOutcome,
};
use super::kill_reward::{DrainOutcome, KillReleaseMark, ReleaseEnd};
use super::native_activity::{
    NativeActivityCommit, NativeActivityOutcome, activity_continuity, finish_activity_kills,
    seal_activity_kills,
};
use super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome, ActivityReturnPlacement,
    ActivityReturnReason, ActivityReturnReceipt, ActivityReturnRequest,
};
use crate::durability::character_last_position::LastVitals;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ActivityTransferMember, ActivityTransferReservation, ActivityTransferredActor, CharacterLease,
    ControlLossMark, GameSessionId, GameSessionState, MovementLocalPosition, RuntimeScopeRefV1,
};
use std::sync::Arc;

/// Client commands and autonomous lifecycle occurrences occupy separate keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ActivityReturnAttemptKey {
    session: GameSessionId,
    connection_generation: u64,
    command: Option<u64>,
    server_reason: u8,
    death_occurrence: Option<[u8; 16]>,
}

pub(super) struct NativeActivityReturnAttempt<'a> {
    source: RegisteredRuntime<'a>,
    destination: RegisteredRuntime<'a>,
    admitted: AdmittedSession,
    // The original autonomous trigger keeps its exact retry identity even when
    // a proven native death selects the temple return instead of a room exit.
    trigger_reason: ActivityReturnReason,
    placement: ActivityReturnPlacement,
    request: ActivityReturnRequest,
    physical: ActivityTransferReservation,
    spells: PreparedActivitySpellStates,
    attacks: PreparedActivityAttackStates,
    kill_marks: Vec<KillReleaseMark>,
    receipt: Option<ActivityReturnReceipt>,
    moved: Option<ActivityTransferredActor>,
    states_installed: bool,
    ancillary_installed: bool,
    death_completed: bool,
    outcome: Option<NativeActivityOutcome>,
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn prune_refused_activity_returns(
        &self,
        policy: super::activity_runtime::ActivityRefusalPrune,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .filter(|(key, _)| policy.includes(key.session, key.connection_generation, key.command))
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        for (key, pending) in entries {
            let held = pending.lock().await;
            if held.receipt.is_some()
                || !matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)))
            {
                continue;
            }
            let mut entries = self.activity_runtimes.returns.lock().map_err(|_| ())?;
            if entries
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &pending))
            {
                entries.remove(&key);
            }
        }
        Ok(())
    }

    /// Only fully completed returns may release their retained source owner.
    /// Unknown SQL/native/respawn outcomes block retirement and retain tokens.
    pub(super) async fn prune_completed_activity_returns(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, attempt)| (*key, Arc::clone(attempt)))
            .collect();
        for (key, pending) in attempts {
            let held = pending.lock().await;
            if held.source.scope != scope || held.source.generation != generation {
                continue;
            }
            if held.outcome.is_none() {
                return Err(());
            }
            self.activity_runtimes
                .returns
                .lock()
                .map_err(|_| ())?
                .remove(&key);
        }
        Ok(())
    }

    pub(super) async fn return_native_activity(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        reason: ActivityReturnReason,
    ) -> Result<NativeActivityOutcome, ()> {
        if command_id == 0
            || matches!(
                reason,
                ActivityReturnReason::Death { .. }
                    | ActivityReturnReason::OwnerLost
                    | ActivityReturnReason::IdleLogout
                    | ActivityReturnReason::Abandoned { .. }
                    | ActivityReturnReason::CapabilityMismatch { .. }
            )
        {
            return Ok(NativeActivityOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        self.prepare_native_activity_return(admitted, Some(command_id), reason)
            .await
    }

    /// The autonomous owner supplies only a reason; the retained return UUID is
    /// the server occurrence. No CommandId or CommandResult is manufactured.
    pub(super) async fn return_due_activity(
        &self,
        admitted: AdmittedSession,
        reason: ActivityReturnReason,
    ) -> Result<NativeActivityOutcome, ()> {
        if !matches!(
            reason,
            ActivityReturnReason::Timeout
                | ActivityReturnReason::Defeated
                | ActivityReturnReason::Death { .. }
                | ActivityReturnReason::Logout
                | ActivityReturnReason::IdleLogout
                | ActivityReturnReason::Abandoned { .. }
                | ActivityReturnReason::CapabilityMismatch { .. }
        ) {
            return Ok(NativeActivityOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        self.prepare_native_activity_return(admitted, None, reason)
            .await
    }

    /// Resolve retained return tokens before a socket-end handler records loss.
    /// A lost scope acknowledgement cannot make the old Instance session live.
    pub(super) async fn settle_native_return_connection(
        &self,
        admitted: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let admitted = self.activity_runtimes.current_connection(admitted);
        let attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            let held = pending.lock().await;
            let belongs = held.admitted.game_session_id == admitted.game_session_id
                && held.admitted.controller == admitted.controller
                && held.admitted.item_fence == admitted.item_fence;
            drop(held);
            if !belongs {
                continue;
            }
            if let NativeActivityOutcome::Committed(commit) =
                self.finish_native_activity_return(pending).await?
            {
                self.publish_activity_commit(&commit)?;
                return Ok(self.activity_runtimes.current_connection(admitted));
            }
            // A prior definite refusal may belong to this same connection.
            // It cannot hide a later retained attempt whose commit is unknown;
            // HashMap iteration order is not a semantic ordering of commands.
        }
        Ok(admitted)
    }

    pub(super) async fn drain_native_return_attempts(&self) -> Result<(), ()> {
        let attempts: Vec<_> = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        let mut complete = true;
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            match self.finish_native_activity_return(pending).await {
                Ok(NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit)?
                }
                Ok(NativeActivityOutcome::Refused(_)) => {}
                Err(()) => complete = false,
            }
        }
        if complete { Ok(()) } else { Err(()) }
    }

    async fn prepare_native_activity_return(
        &self,
        admitted: AdmittedSession,
        command: Option<u64>,
        reason: ActivityReturnReason,
    ) -> Result<NativeActivityOutcome, ()> {
        let refuse = |reason| Ok(NativeActivityOutcome::Refused(reason));
        if matches!(reason, ActivityReturnReason::OwnerLost) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(fence) = admitted.item_fence else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        let key = ActivityReturnAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
            server_reason: if command.is_some() {
                0
            } else {
                match reason {
                    ActivityReturnReason::Timeout => 2,
                    ActivityReturnReason::Defeated => 3,
                    ActivityReturnReason::Death { .. } => 5,
                    ActivityReturnReason::Logout => 6,
                    ActivityReturnReason::IdleLogout => 7,
                    ActivityReturnReason::Abandoned { .. } => 8,
                    ActivityReturnReason::CapabilityMismatch { .. } => 9,
                    _ => return refuse(ActivityRefusal::InvalidInput),
                }
            },
            death_occurrence: match reason {
                ActivityReturnReason::Death { occurrence } => Some(occurrence),
                _ => None,
            },
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        // A cancelled socket may still own an undecided recovery COMMIT.
        // Its admission-serialized reconcile must finish before moving state.
        if self.pending_activity_recovery(admitted.game_session_id) {
            return Err(());
        }
        let existing = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = existing {
            let held = pending.lock().await;
            if held.trigger_reason != reason
                || held.admitted.controller != admitted.controller
                || held.admitted.item_fence != admitted.item_fence
            {
                return refuse(ActivityRefusal::ReplayConflict);
            }
            let retry_server_refusal = command.is_none()
                && held.receipt.is_none()
                && matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)));
            drop(held);
            if retry_server_refusal {
                let mut entries = self.activity_runtimes.returns.lock().map_err(|_| ())?;
                if entries
                    .get(&key)
                    .is_some_and(|current| Arc::ptr_eq(current, &pending))
                {
                    entries.remove(&key);
                }
            } else {
                return self.finish_native_activity_return(pending).await;
            }
        }
        let Some(source) = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let (current, now) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| ())?;
        if super::item_fence_of(current) != Some(fence) {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        let online = current.session_state() == GameSessionState::Active;
        if online {
            if admitted
                .controller
                .is_none_or(|controller| Some(controller.transport) != current.current_transport())
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
        } else if current.session_state() != GameSessionState::Reconnectable
            || command.is_some()
            || current.current_transport().is_some()
            || current.current_control_loss_epoch().is_none()
            || current.current_original_grace_deadline().is_none()
        {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        if reason == ActivityReturnReason::IdleLogout {
            use super::connection::FreshAdmissionAuthority;
            let idle_now = self.gameplay_idle_now();
            if command.is_some()
                || !online
                || admitted.continuity.gameplay_idle.elapsed(idle_now)
                    < std::time::Duration::from_millis(self.gameplay_idle_policy().removal_after_ms)
            {
                return refuse(ActivityRefusal::NotDue);
            }
        }
        match reason {
            ActivityReturnReason::Abandoned { transport }
                if command.is_some()
                    || !online
                    || current.current_transport() != Some(transport)
                    || admitted
                        .controller
                        .is_none_or(|controller| controller.transport != transport) =>
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            ActivityReturnReason::CapabilityMismatch { epoch }
                if command.is_some()
                    || online
                    || current.current_control_loss_epoch() != Some(epoch)
                    || current
                        .current_original_grace_deadline()
                        .is_none_or(|deadline| deadline <= now) =>
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            _ => {}
        }
        if reason == ActivityReturnReason::Logout
            && command.is_none()
            && (online
                || current
                    .current_original_grace_deadline()
                    .is_none_or(|deadline| deadline > now))
        {
            return refuse(ActivityRefusal::NotDue);
        }
        let trigger_reason = reason;
        let source_view = self.runtime_view(&source);
        // A disconnected participant has no online respawn cadence. Preserve
        // the original transport/epoch/deadline checks above before choosing a
        // proven Death return; a stale release must not inherit control of a
        // new Channel successor merely because the actor happens to be dead.
        let reason = if command.is_none() && !matches!(reason, ActivityReturnReason::Death { .. }) {
            let native_death = if let Some(actor) = admitted.runtime_actor {
                let runtime = source.runtime.lock().await;
                let states = source.states.spell_states.lock().await;
                states.player_death(&runtime, actor, admitted.game_session_id)
            } else {
                None
            };
            if let Some(death) = native_death {
                let actor = admitted.runtime_actor.ok_or(())?;
                // Finish the death and selected Item work with the ordinary
                // training fence. Post-loss equipment is refreshed only after
                // the committed Channel return, before native respawn.
                let (settled, _) = source_view
                    .settle_death(actor, admitted.game_session_id, death, false)
                    .await;
                if settled != super::SessionDeath::Durable {
                    return Err(());
                }
                ActivityReturnReason::Death {
                    occurrence: *death.occurrence.as_bytes(),
                }
            } else {
                reason
            }
        } else {
            reason
        };
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let placement = match reason {
            ActivityReturnReason::Death { occurrence } => {
                owner
                    .read_death_return_placement(self.character, self.holder, fence, occurrence)
                    .await
            }
            _ => {
                owner
                    .read_return_placement(self.character, self.holder, fence)
                    .await
            }
        }
        .map_err(|_| ())?;
        let Some(placement) = placement else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let Some(destination) = self.activity_runtimes.get(
            placement.destination_scope,
            placement.destination_generation.get(),
        ) else {
            return refuse(ActivityRefusal::NoCapacity);
        };
        if !matches!(destination.scope, RuntimeScopeRefV1::Channel { .. }) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let destination_session = SecureIdentifiers.game_session_id().ok_or(())?;
        let return_id = *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes();
        let destination_transport = if online {
            Some(SecureIdentifiers.transport_ref().ok_or(())?)
        } else {
            None
        };
        self.activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .try_reserve(1)
            .map_err(|_| ())?;
        let destination_view = self.runtime_view(&destination);
        // Complete every already captured shared lethal before reading live
        // vitals. The queue seal below closes the capture-to-freeze race.
        let mut principals: std::collections::BTreeSet<_> = source
            .states
            .attack
            .lock()
            .await
            .kills()
            .proficiency_principals(admitted.game_session_id)
            .into_iter()
            .collect();
        principals.insert(admitted.game_session_id);
        for principal in principals {
            if source_view.drain_kill_rewards(principal).await == DrainOutcome::Deferred {
                return Err(());
            }
        }
        // Shared-XP settlement acquires its lane before recipient revisions.
        // Both scope lanes must precede every participant slot here as well.
        let source_first = ScopeKey::new(source.scope, source.generation)
            < ScopeKey::new(destination.scope, destination.generation);
        let (_first_lane, _second_lane) = if source_first {
            (
                source.lane.acquire().await.map_err(|_| ())?,
                destination.lane.acquire().await.map_err(|_| ())?,
            )
        } else {
            (
                destination.lane.acquire().await.map_err(|_| ())?,
                source.lane.acquire().await.map_err(|_| ())?,
            )
        };
        let slot = self.revision_sequencer.acquire(fence.character_id).await;
        let (mut from, mut to) = if source_first {
            let from = source.runtime.lock().await;
            let to = destination.runtime.lock().await;
            (from, to)
        } else {
            let to = destination.runtime.lock().await;
            let from = source.runtime.lock().await;
            (from, to)
        };
        from.owner_fence().map_err(|_| ())?;
        to.owner_fence().map_err(|_| ())?;
        let Some(actor) = admitted.runtime_actor else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let lease = from
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        let control = from
            .player_control_facts(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || online != control.control_loss.is_none()
            || (!online
                && control.control_loss.is_none_or(|loss| {
                    Some(loss.epoch)
                        != current
                            .current_control_loss_epoch()
                            .map(|epoch| epoch.get())
                        || Some(loss.grace_deadline) != current.current_original_grace_deadline()
                }))
            || source
                .states
                .player_trades
                .transitioning(admitted.game_session_id)
            || source.states.newhaven_travel.held(admitted.game_session_id)
            || source
                .states
                .dawnport_trials
                .lock()
                .map_err(|_| ())?
                .contains(&admitted.game_session_id)
        {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        let exit = placement.exit_position;
        if exit.coordinate_frame_digest != to.content_pin().frame_binding_digest()
            || exit.map_revision_digest != to.map_revision_digest()
            || exit.server_artifact_digest != to.content_pin().server_artifact_digest()
        {
            return refuse(ActivityRefusal::ContentChanged);
        }
        let doors = destination.door.lock().await;
        let mut spells = source.states.spell_states.lock().await;
        let mut destination_spells = destination.states.spell_states.lock().await;
        let cell = MovementLocalPosition {
            x: exit.cell.x,
            y: exit.cell.y,
            floor: exit.cell.floor,
        };
        if !super::respawn_cell_admissible(
            destination_view.movement_cells.index(),
            destination_view.movement_cells.scope(),
            destination.scope.world_id(),
            exit.server_artifact_digest,
            doors.blocking_cells(),
            cell,
        ) {
            return refuse(ActivityRefusal::Position);
        }
        let Some((_, _, state)) = spells
            .actors
            .iter()
            .find(|(held, session, _)| *held == actor && *session == admitted.game_session_id)
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        // Preserve explicit live pools at maxima too. A real dead actor still
        // has no LastVitals and follows the separately proven Death branch.
        let current_vitals = state.vitals();
        let vitals = LastVitals::new(current_vitals.health, current_vitals.mana);
        let death = match reason {
            ActivityReturnReason::Death { occurrence } => {
                let Some(death) = spells.player_death(&from, actor, admitted.game_session_id)
                else {
                    return refuse(ActivityRefusal::AuthorityRejected);
                };
                if death.occurrence.as_bytes() != &occurrence {
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
                true
            }
            _ => {
                if vitals.is_none() {
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
                false
            }
        };
        let rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
        let member = ActivityTransferMember {
            source_actor: actor,
            source_session: admitted.game_session_id,
            source_lease: lease,
            destination_session,
            destination_lease: CharacterLease::new(
                lease.character_id(),
                lease.generation().checked_add(1).ok_or(())?,
            )
            .map_err(|_| ())?,
            destination_cell: cell,
        };
        let request = ActivityReturnRequest {
            return_id,
            handoff_id: placement.handoff_id,
            command_id: command,
            source: fence,
            destination_game_session_id: destination_session,
            destination_transport,
            destination_generation: placement.destination_generation,
            reason,
            current_vitals: vitals,
            current_rest: rest,
        };
        let (mut attack, mut destination_attack) = if source_first {
            let attack = source.states.attack.lock().await;
            let destination_attack = destination.states.attack.lock().await;
            (attack, destination_attack)
        } else {
            let destination_attack = destination.states.attack.lock().await;
            let attack = source.states.attack.lock().await;
            (attack, destination_attack)
        };
        if matches!(
            reason,
            ActivityReturnReason::Logout
                | ActivityReturnReason::IdleLogout
                | ActivityReturnReason::Abandoned { .. }
                | ActivityReturnReason::CapabilityMismatch { .. }
        ) && super::held_step(
            &spells,
            &attack,
            actor,
            admitted.game_session_id,
            self.owner_now(),
            super::HOLD_STEP,
        )
        .is_some()
        {
            return refuse(ActivityRefusal::NotDue);
        }
        let pending = {
            let mut attempts = self.activity_runtimes.returns.lock().map_err(|_| ())?;
            attempts.try_reserve(1).map_err(|_| ())?;
            let mut kill_marks = seal_activity_kills(attack.kills(), &[member])?;
            let held = if death {
                spells.prepare_activity_death_states(&mut destination_spells, &[member])
            } else {
                spells.prepare_activity_states(&mut destination_spells, &[member])
            };
            let mut held = match held {
                Ok(held) => held,
                Err(()) => {
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
            };
            let mut held_attacks =
                match attack.prepare_activity_attacks(&from, &mut destination_attack, &[member]) {
                    Ok(value) => value,
                    Err(()) => {
                        spells.abort_activity_states(&mut held);
                        finish_activity_kills(
                            attack.kills(),
                            &mut kill_marks,
                            ReleaseEnd::Retryable,
                        );
                        return refuse(ActivityRefusal::AuthorityRejected);
                    }
                };
            let prepared = if placement.boss_room {
                from.prepare_activity_boss_return(&mut to, &[member])
            } else {
                from.prepare_activity_transfer(&mut to, &[member])
            };
            let physical = match prepared {
                Ok(reserved) => reserved,
                Err(_) => {
                    spells.abort_activity_states(&mut held);
                    attack.abort_activity_attacks(&mut held_attacks);
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::NoCapacity);
                }
            };
            let pending = Arc::new(tokio::sync::Mutex::new(NativeActivityReturnAttempt {
                source: source.clone(),
                destination: destination.clone(),
                admitted,
                trigger_reason,
                placement,
                request,
                physical,
                spells: held,
                attacks: held_attacks,
                kill_marks,
                receipt: None,
                moved: None,
                states_installed: false,
                ancillary_installed: false,
                death_completed: false,
                outcome: None,
            }));
            // From extraction to retention there is no await, so cancellation cannot
            // destroy the sole living actor state or free a possibly committed slot.
            attempts.insert(key, Arc::clone(&pending));
            pending
        };
        drop(destination_attack);
        drop(attack);
        drop(destination_spells);
        drop(spells);
        drop(doors);
        drop(to);
        drop(from);
        drop(slot);
        drop(_second_lane);
        drop(_first_lane);
        self.finish_native_activity_return(pending).await
    }

    async fn finish_native_activity_return(
        &self,
        pending: Arc<tokio::sync::Mutex<NativeActivityReturnAttempt<'a>>>,
    ) -> Result<NativeActivityOutcome, ()> {
        let mut attempt = pending.lock().await;
        if let Some(outcome) = &attempt.outcome {
            return Ok(outcome.clone());
        }
        let source = attempt.source.clone();
        let destination = attempt.destination.clone();
        let source_view = self.runtime_view(&source);
        let destination_view = self.runtime_view(&destination);
        // Shared-XP settlement acquires its lane before recipient revisions.
        // Both scope lanes must precede every participant slot here as well.
        let source_first = ScopeKey::new(source.scope, source.generation)
            < ScopeKey::new(destination.scope, destination.generation);
        let (_first_lane, _second_lane) = if source_first {
            (
                source.lane.acquire().await.map_err(|_| ())?,
                destination.lane.acquire().await.map_err(|_| ())?,
            )
        } else {
            (
                destination.lane.acquire().await.map_err(|_| ())?,
                source.lane.acquire().await.map_err(|_| ())?,
            )
        };
        let _slot = self
            .revision_sequencer
            .acquire(attempt.request.source.character_id)
            .await;
        if attempt.receipt.is_none() {
            let account = super::canonical_uuid(&attempt.placement.account_id);
            let evidence = self
                .evidence
                .refresh_account_held(self.root, self.holder, &account)
                .await
                .map_err(|_| ())?;
            let result = ActivityHandoffOwner::new(self.root.clone())
                .return_participant(self.character, self.holder, attempt.request.clone())
                .await;
            evidence.release().await.map_err(|_| ())?;
            let result = result.map_err(|_| ())?;
            match result {
                ActivityReturnOutcome::Returned(receipt)
                | ActivityReturnOutcome::Replayed(receipt) => {
                    if !return_receipt_matches(&attempt.request, &attempt.placement, &receipt) {
                        return Err(());
                    }
                    attempt.receipt = Some(receipt);
                }
                ActivityReturnOutcome::Refused(reason) => {
                    let (mut from, mut to) = if source_first {
                        let from = source.runtime.lock().await;
                        let to = destination.runtime.lock().await;
                        (from, to)
                    } else {
                        let to = destination.runtime.lock().await;
                        let from = source.runtime.lock().await;
                        (from, to)
                    };
                    let mut spells = source.states.spell_states.lock().await;
                    let mut attack = source.states.attack.lock().await;
                    from.abort_activity_transfer(&mut to, &mut attempt.physical)
                        .map_err(|_| ())?;
                    spells.abort_activity_states(&mut attempt.spells);
                    attack.abort_activity_attacks(&mut attempt.attacks);
                    finish_activity_kills(
                        attack.kills(),
                        &mut attempt.kill_marks,
                        ReleaseEnd::Retryable,
                    );
                    let outcome = NativeActivityOutcome::Refused(reason);
                    attempt.outcome = Some(outcome.clone());
                    return Ok(outcome);
                }
            }
        }
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(attempt.request.destination_game_session_id)
            .await
            .map_err(|_| ())?;
        let fence = super::item_fence_of(current).ok_or(())?;
        if current.current_transport() != attempt.request.destination_transport
            || fence.runtime_scope != destination.scope
            || fence.scope_ownership_generation.get() != destination.generation
            || fence.character_id != attempt.request.source.character_id
            || Some(fence.character_lease_generation)
                != attempt
                    .request
                    .source
                    .character_lease_generation
                    .checked_add(1)
            || fence.connection_generation.get() != 1
        {
            return Err(());
        }
        let loss = if attempt.request.destination_transport.is_some() {
            if current.session_state() != GameSessionState::Active {
                return Err(());
            }
            None
        } else {
            if current.session_state() != GameSessionState::Reconnectable {
                return Err(());
            }
            Some(ControlLossMark {
                epoch: current.current_control_loss_epoch().ok_or(())?.get(),
                grace_deadline: current.current_original_grace_deadline().ok_or(())?,
            })
        };
        if !attempt.states_installed {
            let (mut from, mut to) = if source_first {
                let from = source.runtime.lock().await;
                let to = destination.runtime.lock().await;
                (from, to)
            } else {
                let to = destination.runtime.lock().await;
                let from = source.runtime.lock().await;
                (from, to)
            };
            let mut spells = destination.states.spell_states.lock().await;
            let (attack, mut destination_attack) = if source_first {
                let attack = source.states.attack.lock().await;
                let destination_attack = destination.states.attack.lock().await;
                (attack, destination_attack)
            } else {
                let destination_attack = destination.states.attack.lock().await;
                let attack = source.states.attack.lock().await;
                (attack, destination_attack)
            };
            if attempt.moved.is_none() {
                let moved = from
                    .commit_activity_transfer(&mut to, &mut attempt.physical)
                    .map_err(|_| ())?;
                if moved.len() != 1 {
                    return Err(());
                }
                attempt.moved = moved.first().copied();
            }
            let moved = attempt.moved.ok_or(())?;
            if let Some(mark) = loss {
                to.record_control_loss(moved.destination_actor, moved.destination_session, mark)
                    .map_err(|_| ())?;
            }
            let moved_states = [moved];
            let NativeActivityReturnAttempt {
                spells: held_spells,
                attacks: held_attacks,
                ..
            } = &mut *attempt;
            let attack_commit =
                destination_attack.prepare_activity_attack_commit(held_attacks, &moved_states)?;
            spells.commit_activity_states(held_spells, &moved_states)?;
            attack_commit.commit();
            attempt.states_installed = true;
            finish_activity_kills(
                attack.kills(),
                &mut attempt.kill_marks,
                ReleaseEnd::Committed,
            );
        }
        let moved = attempt.moved.ok_or(())?;
        let old = attempt.admitted;
        let controller = match attempt.request.destination_transport {
            Some(transport) => Some(ControllerBinding {
                transport,
                account_id: old.controller.ok_or(())?.account_id,
            }),
            None => None,
        };
        let next = AdmittedSession {
            game_session_id: moved.destination_session,
            runtime_actor: Some(moved.destination_actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller,
            item_fence: Some(fence),
            continuity: activity_continuity(old.continuity),
            ..old
        };
        if !attempt.ancillary_installed {
            source_view.transfer_activity_session_state(&destination_view, old, next)?;
            attempt.ancillary_installed = true;
        }
        // The destination's existing death owner takes its own lane and revision
        // slot while it places, consumes and refills the transferred dead tuple.
        drop(_slot);
        drop(_second_lane);
        drop(_first_lane);
        if let ActivityReturnReason::Death { occurrence } = attempt.request.reason
            && !attempt.death_completed
        {
            let occurrence =
                crate::durability::character_death::PlayerDeathOccurrence::from_bytes(occurrence)
                    .map_err(|_| ())?;
            if !destination_view
                .complete_activity_death_return(next, occurrence)
                .await
            {
                return Err(());
            }
            // No await separates the successful child completion and this
            // retained mark, so cancellation cannot cause a second refill.
            attempt.death_completed = true;
        }
        if next.controller.is_none() {
            self.activity_runtimes.remember_offline_return(old, next)?;
        }
        let outcome = NativeActivityOutcome::Committed(NativeActivityCommit {
            handoff_id: attempt.request.return_id,
            deadline_unix_ms: None,
            connections: vec![(old, next)],
        });
        attempt.outcome = Some(outcome.clone());
        Ok(outcome)
    }
}

fn return_receipt_matches(
    request: &ActivityReturnRequest,
    placement: &ActivityReturnPlacement,
    receipt: &ActivityReturnReceipt,
) -> bool {
    receipt.return_id == request.return_id
        && receipt.handoff_id == request.handoff_id
        && receipt.character_id.as_bytes() == request.source.character_id.as_bytes()
        && receipt.source_game_session_id == request.source.game_session_id
        && receipt.destination_game_session_id == request.destination_game_session_id
        && receipt.destination_scope == placement.destination_scope
        && receipt.destination_generation == placement.destination_generation
        && Some(receipt.destination_lease_generation)
            == request.source.character_lease_generation.checked_add(1)
        && receipt.destination_transport == request.destination_transport
        && receipt.exit_cell == placement.exit_position.cell
}
