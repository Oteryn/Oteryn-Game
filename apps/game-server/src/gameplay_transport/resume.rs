//! FND-04B §20 same-session reauthenticated recovery of a lost GameSession, composed from
//! the real owners (#822 stage 2, PR 5b).
//!
//! A `ClientResume` names a GameSession this Channel lost control of and carries a Platform
//! `oteryn-reauth-recovery-v1` credential. The owner refreshes the Recovery V2 evidence into
//! S2 custody and verifies the credential against the registered floors. It then drives the
//! complete-reconnect PREPARE and COMMIT through the durable adapter, which revalidates the
//! credential against the same floors fenced in its deciding transaction. Only a durably
//! committed switch restores control of the still-present actor and returns the resumed
//! session. A refused or unproven PREPARE is withdrawn, never left to strand the session.

use super::connection::{AdmissionRefusal, AdmittedSession, ControllerBinding, ResumeAttempt};
use super::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY;
use super::{ComposedFreshAdmission, canonical_uuid};
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::fnd04_verifier::{
    RecoveryAccountSecurityObservationV2, RecoveryCurrentEvidence,
    RecoveryDurabilityEvidenceSourceV2, RecoverySigningTrustObservationV2,
    VerifiedRecoveryDurabilityFactsV2, fresh_grant_signing_key_id, recovery_source_sealed,
};
use crate::foundation::{
    CommandId, CompleteReconnectAuthorizationV1, CompleteReconnectCompletionSourceV1,
    CompleteReconnectCompletionV1, CompleteReconnectCurrentV1,
    CompleteReconnectDurabilityOperationV1, CompleteReconnectFlowV1, CompleteReconnectOutcomeV1,
    CompleteReconnectProofTransitionV1, CompleteReconnectProofV1, CompleteReconnectRequestKindV1,
    CompleteReconnectSnapshotV1, CompleteReconnectSourceV1, ConnectionGeneration,
    Fnd02ReconciliationFenceV1, Fnd04EvidenceError, GameSessionState, ReconnectAttemptRef,
    ReconnectCandidateBindingV1, ReconnectDurabilityErrorV1, ReconnectIdentityV1,
    StateDomainRevisionV1,
};
use std::sync::{Arc, Mutex};

/// Candidate lifetime: the widest the 5 s evidence freshness allows (FND-04B §18).
const CANDIDATE_LIFETIME_SECONDS: i64 = 5;

/// The owning reconnect source: current facts resolved once by the owner before the decision,
/// with the Recovery V2 observations the credential was just verified against. The durable
/// adapter independently rebinds session, loss, claims, runtime guard and budget and
/// revalidates the credential against the registered floors it locks.
struct ChannelResumeSource {
    current: Mutex<CompleteReconnectCurrentV1>,
    signing: RecoverySigningTrustObservationV2,
    security: RecoveryAccountSecurityObservationV2,
}
impl recovery_source_sealed::Sealed for ChannelResumeSource {}
impl RecoveryDurabilityEvidenceSourceV2 for ChannelResumeSource {
    fn signing_trust(
        &self,
        key_id: &str,
        _now: i64,
    ) -> Result<RecoverySigningTrustObservationV2, Fnd04EvidenceError> {
        if key_id != self.signing.key_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.signing.clone())
    }
    fn account_security(
        &self,
        account_id: &str,
        _now: i64,
    ) -> Result<RecoveryAccountSecurityObservationV2, Fnd04EvidenceError> {
        if account_id != self.security.account_id {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.security.clone())
    }
}
impl CompleteReconnectSourceV1 for ChannelResumeSource {
    fn resolve_reconnect(
        &self,
        _identity: &ReconnectIdentityV1,
        _now: i64,
    ) -> Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
        self.current
            .lock()
            .map(|current| current.clone())
            .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)
    }
    fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
        Some(self)
    }
}

/// The durable adapter's report of one PREPARE/COMMIT outcome, handed to the flow once.
struct Completion(Option<CompleteReconnectCompletionV1>);
impl recovery_source_sealed::Sealed for Completion {}
impl CompleteReconnectCompletionSourceV1 for Completion {
    fn take_complete_reconnect_completion(
        &mut self,
        _operation: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<Option<CompleteReconnectCompletionV1>, ReconnectDurabilityErrorV1> {
        Ok(self.0.take())
    }
}

/// Deterministic, non-zero attempt identity of one candidate transport.
fn attempt_ref(attempt: &ResumeAttempt<'_>) -> Option<ReconnectAttemptRef> {
    let bytes = attempt.transport.to_bytes();
    let value = u64::from_be_bytes(bytes[..8].try_into().ok()?);
    ReconnectAttemptRef::new(value.max(1)).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn resume_lost(
        &self,
        attempt: ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let lost = self
            .lost
            .lock()
            .map_err(|_| Unavailable)?
            .get(&attempt.game_session_id)
            .copied()
            .ok_or(Rejected)?;
        let (Some(actor), Some(controller)) = (lost.runtime_actor, lost.controller) else {
            return Err(Rejected);
        };
        // The client can only resume from what the server already sent.
        if attempt.last_applied_server_sequence > lost.continuity.server_sequence {
            return Err(Rejected);
        }
        let token = std::str::from_utf8(attempt.recovery_material).map_err(|_| Rejected)?;
        let key_id = fresh_grant_signing_key_id(token).ok_or(Rejected)?;
        let account_id = canonical_uuid(&controller.account_id);
        self.evidence
            .refresh_recovery(self.root, self.holder, &account_id, &key_id)
            .await
            .map_err(|_| Unavailable)?;

        let store = FreshAdmissionStore::from_root(self.root.clone());
        let session_id = lost.game_session_id;
        let (session, now) = store
            .current_session_at(session_id)
            .await
            .map_err(|_| Unavailable)?;
        let epoch = session.current_control_loss_epoch().ok_or(Rejected)?;
        if session.session_state() != GameSessionState::Reconnectable {
            return Err(Rejected);
        }
        let (loss, loss_decided_at) = store
            .owning_loss(session_id, epoch)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Rejected)?;
        let scope = session.current_runtime_scope();
        let character = session.commit().character_id();
        let rows = AdmissionGuardStore::from_root(self.root.clone())
            .load(&[
                AdmissionAuthorityGuardKeyV1::Account {
                    account_id: account_id.clone(),
                },
                AdmissionAuthorityGuardKeyV1::Character(character),
                AdmissionAuthorityGuardKeyV1::Runtime(scope),
            ])
            .await
            .map_err(|_| Unavailable)?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(Rejected)?;
        let AdmissionAuthorityGuardStateV1::Runtime {
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
            ..
        } = &rows[2].state
        else {
            return Err(Unavailable);
        };
        let recovery = RecoveryCurrentEvidence {
            account_id: account_id.clone(),
            character_id: character,
            world_id: session.commit().world_id(),
            ruleset_revision: ruleset_revision.clone(),
            content_revision: content_revision.clone(),
            map_revision: map_revision.clone(),
            world_policy_revision: world_policy_revision.clone(),
        };
        let subject = RecoveryEvidenceSubject::new(account_id.clone(), key_id.clone())
            .map_err(|_| Rejected)?;
        let verify = || async {
            self.root
                .verify_registered_recovery(self.holder, &subject, token, &recovery)
                .await
                .map_err(|_| Unavailable)?
                .map_err(|_| Rejected)
        };
        let verified: VerifiedRecoveryDurabilityFactsV2 = verify().await?;
        let budget = store
            .recovery_budget(session_id, epoch)
            .await
            .map_err(|_| Unavailable)?;
        let (facts, source_revision) = {
            let runtime = self.runtime.lock().await;
            let facts = runtime
                .player_control_facts(actor, session_id)
                .map_err(|_| Rejected)?;
            (facts, runtime.binding().source_revision())
        };
        if facts.control_loss.map(|mark| mark.epoch) != Some(epoch.get()) {
            return Err(Rejected);
        }
        let attempt_ref = attempt_ref(&attempt).ok_or(Unavailable)?;
        let identity = ReconnectIdentityV1::new(
            session_id,
            attempt_ref,
            &account_id,
            character,
            session.commit().world_id(),
            scope,
        )
        .map_err(|_| Rejected)?;
        let predecessor = session.current_connection_generation();
        let successor = predecessor.get().checked_add(1).ok_or(Unavailable)?;
        let candidate = ReconnectCandidateBindingV1::new(
            session_id,
            attempt_ref,
            ConnectionGeneration::new(successor).map_err(|_| Unavailable)?,
            attempt.transport,
            now.checked_add(CANDIDATE_LIFETIME_SECONDS)
                .ok_or(Unavailable)?,
        )
        .map_err(|_| Rejected)?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(lost.continuity.next_command_id).map_err(|_| Unavailable)?,
            Vec::new(),
            lost.continuity.server_sequence,
            vec![
                StateDomainRevisionV1::new(
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    lost.continuity.spatial_revision,
                )
                .map_err(|_| Unavailable)?,
            ],
        )
        .map_err(|_| Unavailable)?;
        let audit = verified.audit();
        let source = Arc::new(ChannelResumeSource {
            current: Mutex::new(CompleteReconnectCurrentV1 {
                snapshot: CompleteReconnectSnapshotV1 {
                    replacement_anchor: None,
                    predecessor_attempts: Vec::new(),
                    account_presence: loss.observation.account_presence.clone(),
                    protection: loss.observation.protection,
                    loss,
                    loss_decided_at,
                    source_authority: scope,
                    source_revision,
                    accepted_source_revision: source_revision,
                    observed_at: now,
                    session,
                    actor_present: true,
                    runtime_ready: true,
                    placement_identity: facts.placement_identity,
                    placement_revision: facts.placement_revision,
                    budget,
                    candidate,
                    proof_transition: CompleteReconnectProofTransitionV1 {
                        owner: scope,
                        revision: source_revision,
                        accepted_revision: source_revision,
                        observed_at: now,
                        predecessor_session: session_id,
                        predecessor_generation: predecessor.get(),
                        successor_session: session_id,
                        successor_generation: successor,
                        candidate,
                    },
                    fnd02,
                    recovery: recovery.clone(),
                    claims: rows[..2].to_vec(),
                },
                prepared: None,
            }),
            signing: audit.signing.clone(),
            security: audit.security.clone(),
        });
        let authorization = CompleteReconnectAuthorizationV1::authorize(
            source.as_ref(),
            identity.clone(),
            CompleteReconnectProofV1::V2(Box::new(verified)),
            now,
        )
        .map_err(|_| Rejected)?;
        let mut flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|_| Rejected)?;
        let prepare = Arc::new(
            flow.take_request(CompleteReconnectRequestKindV1::Prepare)
                .map_err(|_| Rejected)?,
        );
        let prepared = self
            .decide(&store, prepare, source.clone(), flow.operation())
            .await?;
        if !matches!(prepared, CompleteReconnectOutcomeV1::Prepared { .. }) {
            return Err(Rejected);
        }
        let committed = async {
            let budget = recovery_budget_of(&store, session_id, epoch).await?;
            {
                let mut current = source.current.lock().map_err(|_| Unavailable)?;
                current.snapshot.budget = budget;
                current.prepared = Some(Box::new(flow.operation().clone()));
            }
            flow.accept_completion(&mut Completion(Some(CompleteReconnectCompletionV1 {
                operation: flow.operation().clone(),
                outcome: prepared,
            })))
            .map_err(|_| Unavailable)?;
            // COMMIT is reauthorized from a fresh verification, never from PREPARE's.
            let fresh = verify().await?;
            let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
                flow.operation().recovery.clone(),
                CompleteReconnectProofV1::V2(Box::new(fresh)),
                source.as_ref(),
                now,
            )
            .map_err(|_| Rejected)?;
            flow.resume_prepared(authorization, source.as_ref(), now)
                .map_err(|_| Rejected)?;
            let commit = Arc::new(
                flow.take_request(CompleteReconnectRequestKindV1::Commit)
                    .map_err(|_| Rejected)?,
            );
            self.decide(&store, commit, source.clone(), flow.operation())
                .await
        }
        .await;
        if !matches!(committed, Ok(CompleteReconnectOutcomeV1::Committed { .. })) {
            // Never leave a prepared attempt to strand the session until grace expiry.
            let _ = store.abort_complete_reconnect(&identity).await;
            return Err(committed.err().unwrap_or(Rejected));
        }
        // The durable session is ACTIVE on the candidate connection: the still-present
        // actor is controlled again, from the lost connection's FND-02 continuity.
        self.runtime
            .lock()
            .await
            .restore_control(actor, session_id, epoch.get())
            .map_err(|_| Unavailable)?;
        self.forget_lost(session_id, lost.continuity.connection_generation);
        let mut resumed = lost;
        resumed.controller = Some(ControllerBinding {
            transport: attempt.transport,
            account_id: controller.account_id,
        });
        resumed.continuity.connection_generation = successor;
        Ok(resumed)
    }

    /// One PREPARE or COMMIT through the durable adapter; an unproven outcome is reconciled
    /// from the immutable receipts, never decided twice.
    async fn decide(
        &self,
        store: &FreshAdmissionStore,
        request: Arc<crate::foundation::CompleteReconnectRequestV1>,
        source: Arc<ChannelResumeSource>,
        operation: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<CompleteReconnectOutcomeV1, AdmissionRefusal> {
        match store
            .apply_registered_complete_reconnect(self.holder, request, source)
            .await
        {
            Ok(CompleteReconnectOutcomeV1::Ambiguous) | Err(_) => {
                match store.reconcile_complete_reconnect(operation).await {
                    Ok(Some(completion)) => Ok(completion.outcome),
                    Ok(None) => Err(AdmissionRefusal::Rejected),
                    Err(_) => Err(AdmissionRefusal::Unavailable),
                }
            }
            Ok(outcome) => Ok(outcome),
        }
    }
}

async fn recovery_budget_of(
    store: &FreshAdmissionStore,
    session: crate::foundation::GameSessionId,
    epoch: crate::foundation::ControlLossEpochRefV1,
) -> Result<crate::foundation::RetainedRecoveryBudgetV1, AdmissionRefusal> {
    store
        .recovery_budget(session, epoch)
        .await
        .map_err(|_| AdmissionRefusal::Unavailable)
}
