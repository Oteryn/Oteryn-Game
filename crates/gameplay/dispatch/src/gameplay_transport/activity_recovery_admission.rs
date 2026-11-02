//! Reauthenticate a retained, unused activity successor without recreating its
//! actor or granting ordinary reconnect protection. The durable owner validates
//! S2 evidence; this adapter owns only the existing native state and socket gap.
use super::connection::{
    AdmissionRefusal, AdmittedSession, ControllerBinding, FirstEntryOutcome,
    FreshAdmissionAuthority, GraceExpiryResult, ResumeAttempt,
};
use super::{AdmissionRetryBudget, ComposedFreshAdmission};
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityTransferRecovery};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use crate::foundation::fnd04_verifier::fresh_grant_signing_key_id;
use crate::foundation::{ConnectionGeneration, ControlLossMark, GameSessionState};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

const SERVING_ATTEMPT: u8 = 0;
const ABANDONED_ATTEMPT: u8 = 1;
const FINISHED_ATTEMPT: u8 = 2;

#[cfg(test)]
#[derive(Default)]
pub(super) struct RecoveryCommittedTestBarrier {
    pub(super) entered: tokio::sync::Notify,
    pub(super) proceed: tokio::sync::Notify,
}
#[cfg(test)]
tokio::task_local! {
    pub(super) static RECOVERY_COMMITTED_TEST_BARRIER: Arc<RecoveryCommittedTestBarrier>;
}

/// Inserted before the first possible durable mutation. Cancellation changes
/// only `phase`; the stream exclusion and exact proposed transport stay owned
/// by the registry until an admission-serialized read proves the outcome.
pub(super) struct NativeActivityRecovery {
    prior: AdmittedSession,
    next: AdmittedSession,
    controller: ControllerBinding,
    inherited_loss: Option<ControlLossMark>,
    stream: Mutex<Option<super::session_ingress::ActivityStreamReservation>>,
    phase: AtomicU8,
    installed: AtomicBool,
}

struct RecoverySocketGuard(Arc<NativeActivityRecovery>);
impl Drop for RecoverySocketGuard {
    fn drop(&mut self) {
        let _ = self.0.phase.compare_exchange(
            SERVING_ATTEMPT,
            ABANDONED_ATTEMPT,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

impl NativeActivityRecovery {
    fn key(&self) -> Vec<u8> {
        self.controller.transport.to_bytes().to_vec()
    }
    fn release_stream(&self) -> Result<(), ()> {
        self.stream.lock().map_err(|_| ())?.take();
        Ok(())
    }
    pub(super) fn abandoned(&self) -> bool {
        self.phase.load(Ordering::Acquire) == ABANDONED_ATTEMPT
    }
    pub(super) fn blocks_transfer(&self, session: crate::foundation::GameSessionId) -> bool {
        self.next.game_session_id == session
            && self.phase.load(Ordering::Acquire) != FINISHED_ATTEMPT
            && (self.phase.load(Ordering::Acquire) == SERVING_ATTEMPT
                || !self.installed.load(Ordering::Acquire))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn pending_activity_recovery(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> bool {
        self.activity_runtimes
            .recoveries
            .lock()
            .map_or(true, |pending| {
                pending.values().any(|entry| entry.blocks_transfer(session))
            })
    }

    pub(super) fn abandoned_activity_recoveries(
        &self,
    ) -> Vec<(Vec<u8>, Arc<NativeActivityRecovery>)> {
        self.activity_runtimes.recoveries.lock().map_or_else(
            |_| Vec::new(),
            |entries| {
                entries
                    .iter()
                    .filter(|(_, entry)| entry.abandoned())
                    .map(|(key, entry)| (key.clone(), Arc::clone(entry)))
                    .collect()
            },
        )
    }

    fn finish_activity_recovery_record(
        &self,
        pending: &Arc<NativeActivityRecovery>,
    ) -> Result<(), ()> {
        pending.release_stream()?;
        let mut entries = self.activity_runtimes.recoveries.lock().map_err(|_| ())?;
        let key = pending.key();
        if entries
            .get(&key)
            .is_some_and(|entry| !Arc::ptr_eq(entry, pending))
        {
            return Err(());
        }
        pending.phase.store(FINISHED_ATTEMPT, Ordering::Release);
        entries.remove(&key);
        Ok(())
    }

    /// History selects a candidate only. Verification precedes capability
    /// diagnostics, native controller replacement and any consumed grant.
    pub(super) async fn recover_activity_session(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        attempt: &ResumeAttempt<'_>,
    ) -> Result<Option<AdmittedSession>, AdmissionRefusal> {
        use AdmissionRefusal::{RecoveryCapabilityRequired, Rejected, Unavailable};
        let handoffs = ActivityHandoffOwner::new(self.root.clone());
        let Some(candidate) = budget
            .retry_unavailable(async || {
                handoffs
                    .transfer_recovery(attempt.game_session_id)
                    .await
                    .map_err(|_| Unavailable)
            })
            .await?
        else {
            return Ok(None);
        };
        if candidate.current_session == attempt.game_session_id && candidate.has_owning_loss {
            // The existing epoch, attempt ceiling, protection and command
            // continuity remain exclusively the ordinary reconnect owner's.
            return Ok(None);
        }
        if candidate.started {
            return Err(Rejected);
        }
        let token = std::str::from_utf8(attempt.recovery_material).map_err(|_| Rejected)?;
        let key = fresh_grant_signing_key_id(token).ok_or(Rejected)?;
        self.evidence
            .refresh_recovery(self.root, self.holder, &candidate.current.account_id, &key)
            .await
            .map_err(|_| Unavailable)?;
        let subject = RecoveryEvidenceSubject::new(&candidate.current.account_id, &key)
            .map_err(|_| Rejected)?;
        let verified = self
            .root
            .verify_registered_recovery(self.holder, &subject, token, &candidate.current)
            .await
            .map_err(|_| Unavailable)?
            .map_err(|_| Rejected)?;
        let retained = self
            .activity_runtimes
            .connection(candidate.current_session)
            .ok_or(Rejected)?;
        let scoped = self
            .resolve_runtime_view(retained)
            .await
            .map_err(|_| Unavailable)?;
        let owner = scoped.as_ref().unwrap_or(self);
        if !matches_candidate(retained, &candidate)
            || self.activity_runtimes.account_for(retained) != Some(candidate.account)
        {
            return Err(Rejected);
        }
        if candidate.has_owning_loss {
            // The verified old-session alias may locate an unused successor;
            // its old stream's high-water is never applied to this new stream.
            // resume_lost independently repeats credential and current-loss checks.
            let mut wrote = false;
            let resumed = owner
                .resume_lost(
                    budget,
                    ResumeAttempt {
                        game_session_id: candidate.current_session,
                        recovery_material: attempt.recovery_material,
                        transport: attempt.transport,
                        last_applied_server_sequence: 0,
                        supported_capabilities: attempt.supported_capabilities,
                    },
                    &mut wrote,
                )
                .await?;
            self.activity_runtimes
                .remember(resumed)
                .map_err(|_| Unavailable)?;
            owner.finish_resumed_session(resumed, false).await;
            return Ok(Some(resumed));
        }
        let selected = retained.continuity.selected_capabilities;
        let selection_mismatch = !selected.resumable_with(attempt.supported_capabilities);
        if !owner.required_capabilities.matches_current(
            candidate.runtime_scope,
            candidate.scope_generation.get(),
            &candidate.current.world_policy_revision,
        ) {
            return Err(Rejected);
        }
        let capabilities = owner
            .required_capabilities
            .authenticate_resume_support(attempt.supported_capabilities, selected)
            .map_err(|_| RecoveryCapabilityRequired)?;
        capabilities
            .revalidate(
                candidate.runtime_scope,
                candidate.scope_generation.get(),
                &candidate.current.world_policy_revision,
            )
            .map_err(|_| Rejected)?;
        if !selection_mismatch && capabilities.selected() != selected {
            return Err(Rejected);
        }
        if attempt.game_session_id == candidate.current_session
            && attempt.last_applied_server_sequence > retained.continuity.server_sequence
        {
            return Err(Rejected);
        }
        let Some(_input) = owner.states.input_gate.enter().await else {
            return Err(Unavailable);
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        if self.pending_activity_recovery(candidate.current_session) {
            return Err(Unavailable);
        }
        let (Some(actor), Some(fence)) = (retained.runtime_actor, retained.item_fence) else {
            return Err(Rejected);
        };
        let _revision = owner.revision_sequencer.acquire(fence.character_id).await;
        let _lane = owner.spell_lane_permit().await.ok_or(Unavailable)?;
        let mut runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        runtime.owner_fence().map_err(|_| Rejected)?;
        let control = runtime
            .player_control_facts(actor, retained.game_session_id)
            .map_err(|_| Rejected)?;
        let lease = runtime
            .bound_attacker_lease(actor, retained.game_session_id)
            .map_err(|_| Rejected)?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || runtime.content_pin().server_artifact_digest()
                != candidate.destination_content.server_artifact_digest
            || runtime.content_pin().frame_binding_digest()
                != candidate.destination_content.coordinate_frame_digest
            || runtime.map_revision_digest() != candidate.destination_content.map_revision_digest
            || spells
                .get(&runtime, actor, retained.game_session_id)
                .is_none()
            || spells.is_dead(actor)
            || spells.has_pending_spell_commit(actor, retained.game_session_id)
        {
            return Err(Rejected);
        }
        let (current, now) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(retained.game_session_id)
            .await
            .map_err(|_| Unavailable)?;
        if super::item_fence_of(current) != Some(fence) {
            return Err(Rejected);
        }
        match current.session_state() {
            GameSessionState::Active
                if current.current_control_loss_epoch().is_none()
                    && current.current_transport()
                        == retained.controller.map(|controller| controller.transport)
                    && retained.controller.is_some()
                    && control.control_loss.is_none() => {}
            GameSessionState::Reconnectable
                if retained.controller.is_none()
                    && current.current_transport().is_none()
                    && current
                        .current_original_grace_deadline()
                        .is_some_and(|deadline| deadline > now)
                    && control.control_loss.is_some_and(|mark| {
                        current
                            .current_control_loss_epoch()
                            .is_some_and(|epoch| epoch.get() == mark.epoch)
                            && current.current_original_grace_deadline()
                                == Some(mark.grace_deadline)
                    }) => {}
            _ => return Err(Rejected),
        }
        if selection_mismatch {
            // D449 applies only after every other recovery check. An offline
            // returned Channel owns no transport or independent loss receipt;
            // the existing release writer repeats its sealed Activity/S2 proof.
            if candidate.return_id.is_none()
                || retained.controller.is_some()
                || !matches!(
                    candidate.runtime_scope,
                    crate::foundation::RuntimeScopeRefV1::Channel { .. }
                )
                || current.session_state() != GameSessionState::Reconnectable
            {
                return Err(RecoveryCapabilityRequired);
            }
            let epoch = current.current_control_loss_epoch().ok_or(Rejected)?;
            let store = handoffs
                .offline_return_mismatch_store(
                    owner.character,
                    owner.holder,
                    candidate,
                    verified,
                    attempt.transport,
                    capabilities.current_check(),
                )
                .map_err(|_| Unavailable)?;
            drop(spells);
            drop(runtime);
            drop(_lane);
            drop(_revision);
            let release = owner
                .release_terminal_with_store(
                    retained,
                    super::TerminalRelease::CapabilityMismatch(epoch),
                    store,
                    false,
                )
                .await;
            if release == GraceExpiryResult::Released {
                owner.forget_lost(
                    retained.game_session_id,
                    retained.continuity.connection_generation,
                );
            }
            return Err(match release {
                GraceExpiryResult::Released | GraceExpiryResult::NotApplicable => Rejected,
                GraceExpiryResult::Unknown => Unavailable,
            });
        }
        let stream = owner
            .states
            .command_ingress
            .reserve_activity_recovery(&retained)
            .ok_or(Rejected)?;
        let generation = candidate
            .connection_generation
            .checked_add(1)
            .ok_or(Rejected)?;
        let mut next = retained;
        next.first_entry = FirstEntryOutcome::Reconciled;
        next.controller = Some(ControllerBinding {
            transport: attempt.transport,
            account_id: candidate.account,
        });
        next.continuity.connection_generation = generation;
        next.item_fence = Some(
            crate::durability::item_transfer::CurrentCharacterItemFence {
                connection_generation: ConnectionGeneration::new(generation)
                    .map_err(|_| Rejected)?,
                ..fence
            },
        );
        let pending = Arc::new(NativeActivityRecovery {
            prior: retained,
            next,
            // Validate the required proposed controller before retaining this
            // attempt or submitting its first possible durable mutation.
            controller: next.controller.ok_or(Rejected)?,
            inherited_loss: control.control_loss,
            stream: Mutex::new(Some(stream)),
            phase: AtomicU8::new(SERVING_ATTEMPT),
            installed: AtomicBool::new(false),
        });
        {
            let mut entries = self
                .activity_runtimes
                .recoveries
                .lock()
                .map_err(|_| Unavailable)?;
            let key = pending.key();
            if entries.contains_key(&key) {
                return Err(Rejected);
            }
            entries.try_reserve(1).map_err(|_| Unavailable)?;
            entries.insert(key, Arc::clone(&pending));
        }
        let guard = RecoverySocketGuard(Arc::clone(&pending));
        let recovered = handoffs
            .recover_transfer(
                self.character,
                self.holder,
                candidate,
                verified,
                attempt.transport,
                capabilities.current_check(),
            )
            .await
            .map_err(|_| Unavailable)?;
        #[cfg(test)]
        if let Ok(barrier) = RECOVERY_COMMITTED_TEST_BARRIER.try_with(Arc::clone) {
            barrier.entered.notify_one();
            barrier.proceed.notified().await;
        }
        if !matches_candidate(next, &recovered) {
            return Err(Unavailable);
        }
        owner
            .install_activity_recovery(&pending, &mut runtime, &mut spells)
            .map_err(|_| Unavailable)?;
        drop(spells);
        drop(runtime);
        drop(_lane);
        drop(_revision);
        drop(_transition);
        // Keep the cancellation record across the existing reconnect follow-up
        // awaits. Its serving phase still excludes a concurrent room return.
        owner.finish_resumed_session(next, false).await;
        self.finish_activity_recovery_record(&pending)
            .map_err(|_| Unavailable)?;
        drop(guard);
        Ok(Some(next))
    }

    fn install_activity_recovery(
        &self,
        pending: &NativeActivityRecovery,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        spells: &mut super::actor_spell::ChannelSpellStates,
    ) -> Result<(), ()> {
        let actor = pending.next.runtime_actor.ok_or(())?;
        let session = pending.next.game_session_id;
        if let Some(mark) = pending.inherited_loss {
            // This is adoption of an inherited awaiting-controller state, not
            // CompleteReconnect; never open the post-loss protection window.
            runtime
                .restore_control(actor, session, mark.epoch)
                .map_err(|_| ())?;
            if !spells.is_dead(actor) && !spells.resume(runtime, actor, session, self.owner_now()) {
                return Err(());
            }
        } else if runtime
            .player_control_facts(actor, session)
            .map_err(|_| ())?
            .control_loss
            .is_some()
        {
            return Err(());
        }
        self.activity_runtimes.remember(pending.next)?;
        self.forget_lost(session, pending.prior.continuity.connection_generation);
        pending.installed.store(true, Ordering::Release);
        Ok(())
    }

    /// An issued semantic pass never detaches its transaction into another
    /// task. After the socket future ends, current_session_at acquires the same
    /// EXCLUSIVE admission-table locks as recover_transfer: it waits for a sent
    /// COMMIT/rollback before proving the exact transport or its absence.
    pub(super) async fn reconcile_abandoned_activity_recovery(
        &self,
        pending: Arc<NativeActivityRecovery>,
    ) {
        let mut pause = std::time::Duration::from_millis(25);
        while pending.abandoned() {
            let result = self.reconcile_activity_recovery_once(&pending).await;
            if result.is_ok() {
                return;
            }
            tokio::time::sleep(pause).await;
            pause = pause
                .saturating_mul(2)
                .min(std::time::Duration::from_secs(1));
        }
    }

    async fn reconcile_activity_recovery_once(
        &self,
        pending: &Arc<NativeActivityRecovery>,
    ) -> Result<(), ()> {
        let _transition = self.activity_runtimes.transitions.lock().await;
        if pending.installed.load(Ordering::Acquire) {
            pending.release_stream()?;
            drop(_transition);
            // An earlier cleanup may already have committed the origin return
            // before its final release became known. Resolve that exact retained
            // successor again; the terminal old Instance is not completion.
            return self.release_activity_recovery_socket(pending).await;
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store
            .current_session_at(pending.next.game_session_id)
            .await
            .map_err(|_| ())?;
        let owns = |current: crate::foundation::GameSessionAuthoritySnapshot<
            crate::foundation::AuthenticatedTransportRefV1,
        >| {
            super::item_fence_of(current) == pending.next.item_fence
                && current.session_state() == GameSessionState::Active
                && current.current_transport()
                    == pending
                        .next
                        .controller
                        .map(|controller| controller.transport)
        };
        if !owns(current) {
            // An unchanged or later durable generation proves this abandoned
            // mutation absent/fenced even when its old native owner has retired.
            return self.finish_activity_recovery_record(pending);
        }
        let scoped = self.resolve_runtime_view(pending.next).await?;
        let owner = scoped.as_ref().unwrap_or(self);
        let fence = pending.next.item_fence.ok_or(())?;
        let _revision = owner.revision_sequencer.acquire(fence.character_id).await;
        let _lane = owner.spell_lane_permit().await.ok_or(())?;
        let mut runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        let (current, _) = store
            .current_session_at(pending.next.game_session_id)
            .await
            .map_err(|_| ())?;
        if !owns(current) {
            // The serialized read fences any old issued mutation, including
            // an unchanged predecessor. Never attach a later controller.
            return self.finish_activity_recovery_record(pending);
        }
        owner.install_activity_recovery(pending, &mut runtime, &mut spells)?;
        pending.release_stream()?;
        drop(spells);
        drop(runtime);
        drop(_lane);
        drop(_revision);
        drop(_transition);
        self.release_activity_recovery_socket(pending).await
    }

    async fn release_activity_recovery_socket(
        &self,
        pending: &Arc<NativeActivityRecovery>,
    ) -> Result<(), ()> {
        // The proposed socket really ended. Use its existing exact-transport
        // abandoned release, including combat/death saves and Activity return.
        match self.release_abandoned(pending.next).await {
            GraceExpiryResult::Released | GraceExpiryResult::NotApplicable => {
                self.finish_activity_recovery_record(pending)
            }
            _ => Err(()),
        }
    }
}

fn matches_candidate(admitted: AdmittedSession, candidate: &ActivityTransferRecovery) -> bool {
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    admitted.game_session_id == candidate.current_session
        && fence.game_session_id == candidate.current_session
        && fence.character_id.as_bytes() == candidate.current.character_id.as_bytes()
        && fence.runtime_scope == candidate.runtime_scope
        && actor.runtime_scope() == candidate.runtime_scope
        && fence.scope_ownership_generation == candidate.scope_generation
        && actor.scope_generation() == candidate.scope_generation
        && fence.connection_generation.get() == candidate.connection_generation
        && admitted.continuity.connection_generation == candidate.connection_generation
        && fence.character_lease_generation == candidate.lease_generation
        && admitted.continuity.selected_capabilities.as_slice()
            == candidate.selected_capabilities.as_slice()
}
