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

use super::connection::{
    AdmissionRefusal, AdmittedSession, ControllerBinding, FreshAdmissionAuthority,
    GraceExpiryResult, ResumeAttempt,
};
use super::world_required_capabilities::{
    AuthenticatedSessionCapabilities, SessionCapabilityError,
};
use super::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY;
use super::{AdmissionRetryBudget, ComposedFreshAdmission, canonical_uuid};
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
use oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES;
use oteryn_protocol_oteryn::container_tree::STATE_DOMAIN_CONTAINER_VIEWS;
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, STATE_DOMAIN_OPEN_CONTAINER,
};
use oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY;
use std::sync::{Arc, Mutex};

// Scoped to the polled resume future, so concurrent tests cannot share a barrier.
#[cfg(test)]
#[derive(Default)]
pub(super) struct PrepareTestBarrier {
    pub ready: tokio::sync::Notify,
    pub delay: bool,
    pub entered: tokio::sync::Notify,
    pub release: tokio::sync::Notify,
    pub readback: bool,
    pub committed: tokio::sync::Notify,
    pub readback_release: tokio::sync::Notify,
}
#[cfg(test)]
tokio::task_local! {
    pub(super) static PREPARE_TEST_BARRIER: Arc<PrepareTestBarrier>;
}

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
    /// `wrote` is set before the first step that may write (the capability-mismatch release
    /// or PREPARE): until then a refusal wrote nothing and the resume may be repeated. After
    /// PREPARE, each durable read and an unissued PREPARE or COMMIT pass is retried on a busy
    /// holder under `retry` (NODEBOOT-1), so a busy holder no longer closes the resume.
    pub(super) async fn resume_lost(
        &self,
        retry: &AdmissionRetryBudget<'_>,
        attempt: ResumeAttempt<'_>,
        wrote: &mut bool,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        let mut stage = "retained_loss";
        self.resume_lost_at_stage(retry, attempt, wrote, &mut stage)
            .await
            .inspect_err(|refusal| {
                // Busy-holder attempts retain their existing retry without a log per retry.
                if *refusal != AdmissionRefusal::Unavailable {
                    super::connection::report_resume_refusal(stage, *refusal);
                }
            })
    }

    async fn resume_lost_at_stage(
        &self,
        retry: &AdmissionRetryBudget<'_>,
        attempt: ResumeAttempt<'_>,
        wrote: &mut bool,
        stage: &mut &'static str,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        if self
            .states
            .player_trades
            .transitioning(attempt.game_session_id)
        {
            self.spell_lane_permit().await.ok_or(Unavailable)?;
        }
        let lost = self
            .states
            .lost
            .lock()
            .map_err(|_| Unavailable)?
            .get(&attempt.game_session_id)
            .copied();
        let mut lost = match lost {
            Some(lost) => lost,
            None => {
                // The durable loss commit is visible before the loss task finishes its
                // runtime mark, cleanup and lost-entry publication. This is readiness,
                // not a permanent rejection: let the existing bounded pre-write retry
                // read the completed publication, then prove every resume fence below.
                let session = FreshAdmissionStore::from_root(self.root.clone())
                    .current_session(attempt.game_session_id)
                    .await
                    .map_err(|_| Unavailable)?;
                return Err(
                    if session.session_state() == GameSessionState::Reconnectable {
                        Unavailable
                    } else {
                        Rejected
                    },
                );
            }
        };
        *stage = "retained_controller";
        let (Some(actor), Some(controller)) = (lost.runtime_actor, lost.controller) else {
            return Err(Rejected);
        };
        if !tokio::time::timeout(std::time::Duration::from_secs(10), async {
            self.prepare_grizzly_npc_resume(&lost).await
                && self
                    .reconcile_quest_callback_receipts(lost.game_session_id)
                    .await
        })
        .await
        .unwrap_or(false)
        {
            return Err(Unavailable);
        }
        if !super::hireling::recover_pending_lamp_ingress(self, &lost).await {
            return Err(Unavailable);
        }
        *stage = "command_continuity";
        self.states
            .command_ingress
            .recover_trade_continuity(&mut lost)
            .ok_or(Rejected)?;
        if !self.states.command_ingress.resumable(&lost) {
            return Err(Rejected);
        }
        // The client can only resume from what the server already sent.
        *stage = "server_sequence";
        if attempt.last_applied_server_sequence > lost.continuity.server_sequence {
            return Err(Rejected);
        }
        *stage = "credential_shape";
        let token = std::str::from_utf8(attempt.recovery_material).map_err(|_| Rejected)?;
        let key_id = fresh_grant_signing_key_id(token).ok_or(Rejected)?;
        let account =
            crate::domain::AccountId::from_bytes(controller.account_id).map_err(|_| Rejected)?;
        *stage = "account_ban";
        if self
            .root
            .staff_account_banned(self.character, account)
            .await
            .map_err(|_| Unavailable)?
        {
            return Err(Rejected);
        }
        let store = FreshAdmissionStore::from_root(self.root.clone())
            .with_house_lifecycle_owner(self.holder);
        let session_id = lost.game_session_id;
        let (session, _) = store
            .current_session_at(session_id)
            .await
            .map_err(|_| Unavailable)?;
        *stage = "current_loss_epoch";
        let epoch = session.current_control_loss_epoch().ok_or(Rejected)?;
        if session.session_state() != GameSessionState::Reconnectable {
            return Err(Rejected);
        }
        *stage = "current_session_fence";
        let fence = super::item_fence_of(session).ok_or(Rejected)?;
        if fence.connection_generation.get() != lost.continuity.connection_generation
            || self
                .root
                .staff_lost_session_kicked(self.character, self.holder, fence, account, epoch)
                .await
                .map_err(|_| Unavailable)?
        {
            return Err(Rejected);
        }
        let account_id = canonical_uuid(&controller.account_id);
        self.evidence
            .refresh_recovery(self.root, self.holder, &account_id, &key_id)
            .await
            .map_err(|_| Unavailable)?;

        *stage = "owning_loss";
        let (loss, loss_decided_at) = store
            .owning_loss(session_id, epoch)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Rejected)?;
        let scope = session.current_runtime_scope();
        let character = session.commit().character_id();
        *stage = "authority_claims";
        let rows = AdmissionGuardStore::from_root(self.root.clone())
            .load(&[
                AdmissionAuthorityGuardKeyV1::Account {
                    account_id: account_id.clone(),
                },
                AdmissionAuthorityGuardKeyV1::Character(character),
            ])
            .await
            .map_err(|_| Unavailable)?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(Rejected)?;
        *stage = "runtime_publication";
        let runtime_publication = store
            .recovery_runtime_publication(session_id)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Rejected)?;
        let AdmissionAuthorityGuardKeyV1::Runtime(policy_scope) = &runtime_publication.key else {
            return Err(Unavailable);
        };
        let policy_scope = *policy_scope;
        let AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: policy_generation,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
            ..
        } = &runtime_publication.state
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
        *stage = "recovery_subject";
        let subject = RecoveryEvidenceSubject::new(account_id.clone(), key_id.clone())
            .map_err(|_| Rejected)?;
        let verify = || async {
            self.root
                .verify_registered_recovery(self.holder, &subject, token, &recovery)
                .await
                .map_err(|_| Unavailable)?
                .inspect_err(|error| report_recovery_verification_error(*error))
                .map_err(|_| Rejected)
        };
        *stage = "credential_verification";
        let verified: VerifiedRecoveryDurabilityFactsV2 = verify().await?;
        // PREPARE is decided at a durable time sampled after verification, as COMMIT does with
        // `final_now`: the verifier stamps `verified_at` from its own later clock sample and
        // revalidation refuses any earlier `now`, so a sample taken before `verify()` loses the
        // resume whenever a second boundary falls between the two.
        let (_, now) = store
            .current_session_at(session_id)
            .await
            .map_err(|_| Unavailable)?;
        let budget = store
            .recovery_budget(session_id, epoch)
            .await
            .map_err(|_| Unavailable)?;
        #[cfg(test)]
        if let Ok(barrier) = PREPARE_TEST_BARRIER.try_with(Arc::clone) {
            if barrier.delay {
                // Model a pre-lock read still owning the root on a slow runner.
                let delayed = Arc::clone(&barrier);
                self.root
                    .try_issue_semantic_pass()
                    .map_err(|_| Unavailable)?
                    .run(move |_, _| {
                        Box::pin(async move {
                            delayed.entered.notify_one();
                            delayed.release.notified().await;
                            Ok(())
                        })
                    })
                    .await
                    .map_err(|_| Unavailable)?;
            }
            barrier.ready.notify_one();
        }
        // Activities also use Instance scopes. Only a native House resident
        // resumes through the named-House custody and ACL owner.
        let house_resume = self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, session_id)
            .is_ok();
        *stage = "house_authority";
        if house_resume && !self.validate_house_resume(actor, session_id).await {
            return Err(Rejected);
        }
        *stage = "runtime_control";
        let (facts, source_revision) = {
            let runtime = self.runtime.lock().await;
            let facts = if house_resume {
                let house = runtime
                    .house_actor_state(actor, session_id)
                    .map_err(|_| Rejected)?;
                if house.scope != scope
                    || house.generation != session.current_scope_generation()
                    || house.lease != session.current_character_lease()
                {
                    return Err(Rejected);
                }
                runtime.house_player_control_facts(actor, session_id)
            } else {
                runtime.player_control_facts(actor, session_id)
            }
            .map_err(|_| Rejected)?;
            (facts, runtime.binding().source_revision())
        };
        *stage = "runtime_loss_epoch";
        if facts.control_loss.map(|mark| mark.epoch) != Some(epoch.get()) {
            return Err(Rejected);
        }
        let attempt_ref = attempt_ref(&attempt).ok_or(Unavailable)?;
        *stage = "reconnect_identity";
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
        *stage = "candidate_binding";
        let candidate = ReconnectCandidateBindingV1::new(
            session_id,
            attempt_ref,
            ConnectionGeneration::new(successor).map_err(|_| Unavailable)?,
            attempt.transport,
            // Never outlive the verified evidence or the original grace: the Foundation
            // refuses a candidate deadline past either, and the evidence was observed
            // before `now` was sampled, so `now + 5` can exceed it by a second boundary.
            now.checked_add(CANDIDATE_LIFETIME_SECONDS)
                .ok_or(Unavailable)?
                .min(verified.accepted_deadline())
                .min(loss.observation.original_grace_deadline),
        )
        .map_err(|_| Rejected)?;
        let mut domains = vec![
            StateDomainRevisionV1::new(
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                lost.continuity.spatial_revision,
            )
            .map_err(|_| Unavailable)?,
            // Inert for a session with capability 18: it is never sent domain 2, so this is 0.
            StateDomainRevisionV1::new(
                STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                lost.continuity.overlay_revision,
            )
            .map_err(|_| Unavailable)?,
        ];
        // ITEM-VIEW-1b: with capability 4, the domain 9 and 11 high-water revisions. The
        // handle counter travels in the carried continuity; the resumed snapshot reissues fresh
        // handles above it and every older handle is STALE.
        if lost
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY)
        {
            let item_view = lost.continuity.item_view;
            for (domain, revision) in [
                (
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    item_view.inventory_revision,
                ),
                (STATE_DOMAIN_OPEN_CONTAINER, item_view.container_revision),
            ] {
                domains
                    .push(StateDomainRevisionV1::new(domain, revision).map_err(|_| Unavailable)?);
            }
        }
        // ACHIEVEMENT-0 §5: with capability 8, the cumulative notice revision the resumed
        // connection's snapshot carries (domain 13, after domains 1 and 2).
        if let Some(revision) = lost.continuity.achievement_notice_revision.filter(|_| {
            lost.continuity
                .selected_capabilities
                .domain_selected(STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES)
        }) {
            domains.push(
                StateDomainRevisionV1::new(STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, revision)
                    .map_err(|_| Unavailable)?,
            );
        }
        // BAGS-WIRE-1: with capability 14, the domain 14 high-water revision. Its views closed
        // with the lost connection; the resumed snapshot is empty above it.
        if lost
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CONTAINER_VIEWS)
        {
            domains.push(
                StateDomainRevisionV1::new(
                    STATE_DOMAIN_CONTAINER_VIEWS,
                    lost.continuity.item_view.views_revision,
                )
                .map_err(|_| Unavailable)?,
            );
        }
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(lost.continuity.next_command_id).map_err(|_| Unavailable)?,
            Vec::new(),
            lost.continuity.server_sequence,
            domains,
        )
        .map_err(|_| Unavailable)?;
        let audit = verified.audit();
        let grace_deadline = loss.observation.original_grace_deadline;
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
        *stage = "prepare_authorization";
        let authorization = CompleteReconnectAuthorizationV1::authorize(
            source.as_ref(),
            identity.clone(),
            CompleteReconnectProofV1::V2(Box::new(verified)),
            now,
        )
        .inspect_err(|error| report_reconnect_error("prepare_authorization", *error))
        .map_err(|_| Rejected)?;
        *stage = "prepare_flow";
        let mut flow = CompleteReconnectFlowV1::begin(authorization, None)
            .inspect_err(|error| report_reconnect_error("prepare_flow", *error))
            .map_err(|_| Rejected)?;
        // ADMIT0/D1084: only the authenticated and safely classified owner path may
        // reveal a missing requirement. This precedes both PREPARE and the legacy
        // capability-mismatch release: ADMIT0 refusal preserves all current claims.
        // A retained House selection still uses its genuine origin policy
        // publication. Current House ownership/ACL are proved independently;
        // the origin generation is never installed as House session authority.
        *stage = "capability_policy";
        if !self.required_capabilities.matches_current(
            policy_scope,
            *policy_generation,
            &recovery.world_policy_revision,
        ) {
            return Err(Rejected);
        }
        *stage = "capability_support";
        let capabilities = self
            .required_capabilities
            .authenticate_resume_support(
                attempt.supported_capabilities,
                lost.continuity.selected_capabilities,
            )
            .map_err(|error| match error {
                SessionCapabilityError::MissingSupport(_) => {
                    AdmissionRefusal::RecoveryCapabilityRequired
                }
                _ => Unavailable,
            })?;
        *stage = "capability_revalidation";
        capabilities
            .revalidate(
                policy_scope,
                *policy_generation,
                &recovery.world_policy_revision,
            )
            .map_err(|_| Rejected)?;
        if capabilities
            .selected()
            .as_slice()
            .iter()
            .any(|id| !lost.continuity.selected_capabilities.contains(*id))
        {
            // Current mandatory requirements cannot widen a retained negotiation.
            return Err(AdmissionRefusal::RecoveryCapabilityRequired);
        }
        // CAP-NEG-1: the resumed session keeps its original selection and never widens it; a
        // client that no longer supports a selected capability falls back to fresh admission.
        // D449: only a resume that passed every other check (the verified recovery credential,
        // the same account, character and World, the session RECONNECTABLE at this epoch within
        // its original grace, the runtime facts, and the complete authorization of its
        // candidate, attempt budget and current claims) ends the lost session in a terminal
        // release with no successor, so that fresh admission finds the character free. Any
        // other refused resume, an exhausted attempt budget included, releases nothing.
        *stage = "capability_continuity";
        if !lost
            .continuity
            .selected_capabilities
            .resumable_with(attempt.supported_capabilities)
        {
            if now >= grace_deadline {
                return Err(Rejected);
            }
            *wrote = true;
            let release = self.release_capability_mismatch(lost, epoch).await;
            return Err(capability_mismatch_refusal(release));
        }
        *stage = "prepare_request";
        let prepare = Arc::new(
            flow.take_request(CompleteReconnectRequestKindV1::Prepare)
                .inspect_err(|error| report_reconnect_error("prepare_request", *error))
                .map_err(|_| Rejected)?,
        );
        // The resumed session as the candidate connection would serve it.
        let mut resumed = lost;
        // The authenticated boot profile checks requirements, but does not contain
        // dynamically offered mechanics. Retain the original negotiated set exactly.
        resumed.controller = Some(ControllerBinding {
            transport: attempt.transport,
            account_id: controller.account_id,
        });
        resumed.continuity.connection_generation = successor;
        // C2: the lost connection's item fence names its own generation; it is refreshed below
        // from the committed resume, never carried over.
        resumed.item_fence = None;
        *wrote = true;
        *stage = "prepare_decision";
        let prepared = match self
            .decide(
                retry,
                &store,
                prepare,
                source.clone(),
                flow.operation(),
                &capabilities,
            )
            .await
        {
            Ok(outcome @ CompleteReconnectOutcomeV1::Prepared { .. }) => outcome,
            refused => {
                // A PREPARE that landed but could not be proven must not stay named.
                let _ = store.abort_complete_reconnect(&identity).await;
                return Err(refused.err().unwrap_or(Rejected));
            }
        };
        let committed = async {
            *stage = "commit_budget";
            let budget = retry
                .retry_unavailable(async || recovery_budget_of(&store, session_id, epoch).await)
                .await?;
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
            // COMMIT is reauthorized from a fresh verification at a fresh durable time,
            // never from PREPARE's capability.
            *stage = "commit_verification";
            let fresh = retry.retry_unavailable(async || verify().await).await?;
            let (_, final_now) = retry
                .retry_unavailable(async || {
                    store
                        .current_session_at(session_id)
                        .await
                        .map_err(|_| Unavailable)
                })
                .await?;
            *stage = "commit_authorization";
            let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
                flow.operation().recovery.clone(),
                CompleteReconnectProofV1::V2(Box::new(fresh)),
                source.as_ref(),
                final_now,
            )
            .inspect_err(|error| report_reconnect_error("commit_authorization", *error))
            .map_err(|_| Rejected)?;
            // The deciding store will independently resolve and check the locked
            // current runtime again; this immutable support never supplies authority.
            *stage = "commit_flow";
            flow.resume_prepared(authorization, source.as_ref(), final_now)
                .inspect_err(|error| report_reconnect_error("commit_flow", *error))
                .map_err(|_| Rejected)?;
            *stage = "commit_request";
            let commit = Arc::new(
                flow.take_request(CompleteReconnectRequestKindV1::Commit)
                    .inspect_err(|error| report_reconnect_error("commit_request", *error))
                    .map_err(|_| Rejected)?,
            );
            *stage = "commit_decision";
            self.decide(
                retry,
                &store,
                commit,
                source.clone(),
                flow.operation(),
                &capabilities,
            )
            .await
        }
        .await;
        match committed {
            Ok(CompleteReconnectOutcomeV1::Committed { .. }) => {}
            Err(AdmissionRefusal::Unavailable) => {
                // The COMMIT may have landed without proof. If it did, the session is ACTIVE on
                // a candidate that will never serve: release it (only that exact transport),
                // never strand it. If it did not, withdraw the prepared attempt.
                let _ = self.release_abandoned(resumed).await;
                let _ = store.abort_complete_reconnect(&identity).await;
                return Err(Unavailable);
            }
            refused => {
                let _ = store.abort_complete_reconnect(&identity).await;
                return Err(refused.err().unwrap_or(Rejected));
            }
        }
        // The durable session is ACTIVE on the candidate connection: the still-present actor
        // is controlled again, from the lost connection's FND-02 continuity. The durable
        // switch stands even if the runtime mark could not be cleared.
        *stage = "committed_house_authority";
        if house_resume && !self.validate_house_resume(actor, session_id).await {
            // Current ACL or cell facts may have changed while PREPARE/COMMIT
            // ran. Retain the native loss/fence until the ordinary terminal
            // owner proves release; never reactivate on the stale House cell.
            let released = self.release_abandoned_session(resumed).await;
            return Err(if released == GraceExpiryResult::Released {
                Rejected
            } else {
                Unavailable
            });
        }
        {
            let mut runtime = self.runtime.lock().await;
            if house_resume {
                runtime
                    .restore_house_control_after_committed_reentry(
                        actor,
                        session_id,
                        epoch.get(),
                        self.owner_now().get(),
                    )
                    .map_err(|_| Unavailable)?;
            } else {
                let _ = runtime.restore_control_after_committed_reentry(
                    actor,
                    session_id,
                    epoch.get(),
                    self.owner_now().get(),
                );
            }
            // SPELL-D8 §8.2: the Serene initialization evaluation runs in this owner step,
            // before the resumed actor's first command. An actor without spell state has none.
            let mut states = self.states.spell_states.lock().await;
            if house_resume {
                let _ = states.resume_house(&runtime, actor, session_id, self.owner_now());
            } else {
                let _ = states.resume(&runtime, actor, session_id, self.owner_now());
            }
        }
        self.forget_lost(session_id, lost.continuity.connection_generation);
        // C2: the resumed session's item fence, from a current durable read of the session the
        // COMMIT made ACTIVE on this candidate connection. An unreadable or already moved
        // session leaves it `None`: the resume stands, and a chest `USE` is refused.
        #[cfg(test)]
        if let Ok(barrier) = PREPARE_TEST_BARRIER.try_with(Arc::clone)
            && barrier.readback
        {
            barrier.committed.notify_one();
            barrier.readback_release.notified().await;
        }
        // MAINRED14: a busy holder is not missing authority. Without this fence the first
        // resumed command cannot reconcile retained NPC work and disconnects after its
        // snapshot. Retry only the read, never the already committed reconnect.
        let current = AdmissionRetryBudget::for_command(self.root)
            .retry_unavailable(async || {
                store
                    .current_session_at(session_id)
                    .await
                    .map_err(|error| match error {
                        crate::durability::DurabilityError::RootUnavailable => Unavailable,
                        _ => Rejected,
                    })
            })
            .await;
        if let Ok((current, _)) = current
            && current.session_state() == GameSessionState::Active
            && current.current_connection_generation().get() == successor
            && current.current_transport() == Some(attempt.transport)
        {
            resumed.item_fence = super::item_fence_of(current);
        }
        Ok(resumed)
    }

    /// One PREPARE or COMMIT through the durable adapter; an unproven outcome is reconciled
    /// from the immutable receipts, never decided twice. The reconciliation read is retried
    /// on a busy holder under `retry`. A pass that found the holder busy was never issued: once
    /// the receipts prove it absent, it is applied again under `retry` (NODEBOOT-1). An issued
    /// pass that left no receipt is final, as before.
    async fn decide(
        &self,
        retry: &AdmissionRetryBudget<'_>,
        store: &FreshAdmissionStore,
        request: Arc<crate::foundation::CompleteReconnectRequestV1>,
        source: Arc<ChannelResumeSource>,
        operation: &CompleteReconnectDurabilityOperationV1,
        capabilities: &AuthenticatedSessionCapabilities,
    ) -> Result<CompleteReconnectOutcomeV1, AdmissionRefusal> {
        loop {
            let unissued = match store
                .apply_registered_complete_reconnect(
                    self.holder,
                    request.clone(),
                    source.clone(),
                    capabilities.current_check(),
                )
                .await
                .inspect_err(|error| {
                    if !matches!(error, crate::durability::DurabilityError::RootUnavailable) {
                        let stage = match request.kind() {
                            CompleteReconnectRequestKindV1::Prepare => "prepare_store",
                            CompleteReconnectRequestKindV1::Commit => "commit_store",
                            CompleteReconnectRequestKindV1::Reconcile => "reconcile_store",
                        };
                        report_reconnect_store_error(stage, error);
                    }
                }) {
                Ok(CompleteReconnectOutcomeV1::Ambiguous) => false,
                Err(error) => matches!(error, crate::durability::DurabilityError::RootUnavailable),
                Ok(outcome) => return Ok(outcome),
            };
            let reconciled = retry
                .retry_unavailable(async || {
                    store
                        .reconcile_complete_reconnect(operation)
                        .await
                        .map_err(|_| AdmissionRefusal::Unavailable)
                })
                .await?;
            match reconciled {
                Some(completion) => return Ok(completion.outcome),
                None if !unissued => return Err(AdmissionRefusal::Rejected),
                None => {
                    if !retry
                        .retries_unavailable(AdmissionRefusal::Unavailable)
                        .await
                    {
                        return Err(AdmissionRefusal::Unavailable);
                    }
                }
            }
        }
    }
}

// [DBG-NODEBOOT]: every class below is an exhaustive, data-free label. Never format the
// source error: its nested fields may gain private data independently of this diagnostic.
fn report_reconnect_error(stage: &'static str, error: ReconnectDurabilityErrorV1) {
    let class = match error {
        ReconnectDurabilityErrorV1::InvalidRecord => "invalid_record",
        ReconnectDurabilityErrorV1::IdempotencyConflict => "idempotency_conflict",
        ReconnectDurabilityErrorV1::AttemptCapacityExceeded => "attempt_capacity_exceeded",
        ReconnectDurabilityErrorV1::ConcurrentPrepared => "concurrent_prepared",
        ReconnectDurabilityErrorV1::CompletionMismatch => "completion_mismatch",
        ReconnectDurabilityErrorV1::InvalidPhase => "invalid_phase",
        ReconnectDurabilityErrorV1::StaleAuthority => "stale_authority",
        ReconnectDurabilityErrorV1::DeadlineExpired => "deadline_expired",
        ReconnectDurabilityErrorV1::ReconciliationMismatch => "reconciliation_mismatch",
    };
    super::operator_event(&format!(
        "[DBG-NODEBOOT] event=resume_refused stage={stage} class={class}"
    ));
}

fn report_reconnect_store_error(stage: &'static str, error: &crate::durability::DurabilityError) {
    use crate::durability::DurabilityError as E;
    let class = match error {
        E::AdmissionGameSessionLedgerExhausted => "session_ledger_exhausted",
        E::SessionUseAuthorization(_) => "session_use_authorization",
        E::Unavailable => "durability_unavailable",
        E::Database(_) => "database",
        E::Migration(_) => "migration",
        E::SchemaIncompatible(_) => "schema_incompatible",
        E::InvalidConfiguration => "invalid_configuration",
        E::RootUnavailable => "root_unavailable",
        E::RootPassDeadlineExceeded => "root_pass_deadline_exceeded",
        E::CommitRejected => "commit_rejected",
        E::CommitOutcomeUnknown => "commit_outcome_unknown",
        E::RootTaskFailed => "root_task_failed",
        E::InvalidStoredState => "invalid_stored_state",
    };
    super::operator_event(&format!(
        "[DBG-NODEBOOT] event=resume_refused stage={stage} class={class}"
    ));
}

fn report_recovery_verification_error(
    error: crate::foundation::fnd04_verifier::Fnd04ConsumerError,
) {
    use crate::foundation::fnd04_verifier::Fnd04ConsumerError as E;
    let class = match error {
        E::RecoveryMalformed => "recovery_malformed",
        E::RecoveryAuthenticationFailed => "recovery_authentication_failed",
        E::RecoveryBindingMismatch => "recovery_binding_mismatch",
        E::RecoveryRevisionUnsupported => "recovery_revision_unsupported",
        E::RecoveryNotYetValid => "recovery_not_yet_valid",
        E::RecoveryExpired => "recovery_expired",
        E::RecoverySecurityEvidenceStale => "recovery_security_evidence_stale",
        E::RecoverySecurityStateRevoked => "recovery_security_state_revoked",
        E::RecoveryWorldStale => "recovery_world_stale",
        E::FreshMalformed
        | E::FreshAuthenticationFailed
        | E::FreshBindingMismatch
        | E::FreshRevisionUnsupported
        | E::FreshNotYetValid
        | E::FreshExpired
        | E::FreshSecurityEvidenceStale
        | E::FreshSecurityStateRevoked
        | E::FreshAccountCharacterConflict
        | E::FreshWorldStale
        | E::FreshRouteStale
        | E::FreshRuntimeStale => "unexpected_fresh_verification",
    };
    super::operator_event(&format!(
        "[DBG-NODEBOOT] event=resume_refused stage=credential_verification class={class}"
    ));
}

/// D449: the refusal of a resume refused for the capability mismatch alone. Only a release that
/// ended the session (or found it no longer reconnectable) is final. An unproven release is
/// `Unavailable`: the lost entry stays, and a durable row still holding the lease lifts the
/// release's fence, so a retry repeats the release; grace expiry stays the backstop.
fn capability_mismatch_refusal(release: GraceExpiryResult) -> AdmissionRefusal {
    match release {
        GraceExpiryResult::Released | GraceExpiryResult::NotApplicable => {
            AdmissionRefusal::Rejected
        }
        GraceExpiryResult::Unknown => AdmissionRefusal::Unavailable,
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// #162 5870253781 (r4122215795, reconnect fence P1): `resume_lost`'s FND-02
    /// reconciliation fence must carry both domain 1 (`WORLD_SPATIAL_VISIBILITY`) and domain 2
    /// (`WORLD_OBJECT_OVERLAY`) revisions after a resume, in ascending domain-id order —
    /// `Fnd02ReconciliationFenceV1::new` itself requires strictly ascending order and refuses
    /// otherwise. This exercises the exact two-entry construction `resume_lost` builds, with the
    /// same domain constants and field types it reads from `lost.continuity`.
    #[test]
    fn reconciliation_fence_carries_both_spatial_and_overlay_domains() {
        let fence = Fnd02ReconciliationFenceV1::new(
            CommandId::new(7).expect("command id"),
            Vec::new(),
            9,
            vec![
                StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, 3)
                    .expect("spatial domain revision"),
                StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_OBJECT_OVERLAY, 2)
                    .expect("overlay domain revision"),
            ],
        )
        .expect("reconciliation fence");
        let domains = fence.domain_revisions();
        assert_eq!(domains.len(), 2);
        assert_eq!(
            domains[0].domain_id(),
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
        );
        assert_eq!(domains[0].revision(), 3);
        assert_eq!(domains[1].domain_id(), STATE_DOMAIN_WORLD_OBJECT_OVERLAY);
        assert_eq!(domains[1].revision(), 2);
    }

    /// The reverse (descending domain-id) order the fence would carry if the overlay entry were
    /// ever placed before the spatial one is refused, proving the ordering in `resume_lost`
    /// matters and is exercised, not incidental.
    #[test]
    fn reconciliation_fence_refuses_descending_domain_order() {
        assert!(
            Fnd02ReconciliationFenceV1::new(
                CommandId::new(1).expect("command id"),
                Vec::new(),
                1,
                vec![
                    StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_OBJECT_OVERLAY, 1)
                        .expect("overlay domain revision"),
                    StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, 1)
                        .expect("spatial domain revision"),
                ],
            )
            .is_err()
        );
    }

    /// ITEM-VIEW-1b: with capabilities 4 and 8 the fence carries domains 1, 2, 9, 11 and 13, in
    /// the ascending order `resume_lost` pushes them.
    #[test]
    fn reconciliation_fence_carries_the_item_view_high_water_revisions_in_order() {
        let fence = Fnd02ReconciliationFenceV1::new(
            CommandId::new(7).expect("command id"),
            Vec::new(),
            9,
            [
                (STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, 3),
                (STATE_DOMAIN_WORLD_OBJECT_OVERLAY, 2),
                (STATE_DOMAIN_CHARACTER_INVENTORY, 5),
                (STATE_DOMAIN_OPEN_CONTAINER, 4),
                (STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, 1),
            ]
            .into_iter()
            .map(|(domain, revision)| {
                StateDomainRevisionV1::new(domain, revision).expect("domain revision")
            })
            .collect(),
        )
        .expect("reconciliation fence");
        let domains: Vec<_> = fence
            .domain_revisions()
            .iter()
            .map(|domain| (domain.domain_id(), domain.revision()))
            .collect();
        assert_eq!(domains, [(1, 3), (2, 2), (9, 5), (11, 4), (13, 1)]);
    }

    /// D449 (#1708 Codex P2): only a proven release ends the refused resume; an unproven one
    /// stays retryable (`Unavailable`), never a final `Rejected`.
    #[test]
    fn capability_mismatch_refusal_is_final_only_for_a_proven_release() {
        assert_eq!(
            capability_mismatch_refusal(GraceExpiryResult::Released),
            AdmissionRefusal::Rejected
        );
        assert_eq!(
            capability_mismatch_refusal(GraceExpiryResult::NotApplicable),
            AdmissionRefusal::Rejected
        );
        assert_eq!(
            capability_mismatch_refusal(GraceExpiryResult::Unknown),
            AdmissionRefusal::Unavailable
        );
    }
}
