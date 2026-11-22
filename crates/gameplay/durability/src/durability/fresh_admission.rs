//! Strict historical fresh-operation storage. No decoded value is a live capability.
//! Runtime ceilings follow the accepted DFR registry; codec bounds remain explicit.
mod activity_loss;
mod house_lifecycle;
mod retired_holder_release;
mod terminal_release;
mod training_lineage;
use crate::durability::monster_profile_metrics::query as profile_query;
use super::DurabilityError;
use super::admission_authority_guards::*;
use crate::foundation::admission_authority_publication::*;
use crate::foundation::fnd04_verifier::*;
use crate::foundation::fresh_admission_durability::*;
use crate::foundation::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
pub(super) use retired_holder_release::release_retired_holder_sessions;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, DurabilityError>;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    version: u8,
    payload: &'a str,
}

fn channel_switch_refusal(
    error: &sqlx::Error,
) -> Option<oteryn_protocol_oteryn::channel_switch::SwitchStatus> {
    let error = error
        .as_database_error()?
        .try_downcast_ref::<sqlx::postgres::PgDatabaseError>()?;
    match error.code() {
        "OTH01" => {
            super::channel_switch::preflight_status(error.detail()?.parse().ok()?, None, true, true)
        }
        "OTH02" => super::channel_switch::preflight_status(0, Some(error.detail()?), true, true),
        _ => None,
    }
}

include!("fresh_admission/operation_codec.rs");

/// Owner revalidation run inside a fresh-admission commit transaction.
pub(super) type OwnerRevalidation = Box<
    dyn for<'t, 'c> FnOnce(
            &'t mut sqlx::Transaction<'c, sqlx::Postgres>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<bool>> + Send + 't>,
        > + Send,
>;

/// Admission-owner predicate over independently read, locked CURRENT runtime facts.
/// Prepared/receipt facts must never be substituted for these callback arguments.
/// The callback carries immutable build requirements and authenticated support;
/// it does not supply current scope authority and cannot grant a lease or session.
pub type CurrentCapabilityCheck =
    std::sync::Arc<dyn Fn(RuntimeScopeRefV1, u64, &str) -> bool + Send + Sync>;

include!("fresh_admission/envelope.rs");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshReconciliation {
    Absent,
    Conflict,
    Committed(Box<FreshAdmissionDurableReconciliationSnapshotV1>),
}

/// Inert historical outcome and independently read current snapshot. This is not
/// a registered completion source or permission to activate a controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshLossReconciliation {
    Absent,
    Conflict,
    Committed {
        completion: Box<ControlLossCompletionV1>,
        current: Box<FreshAdmissionDurableReconciliationSnapshotV1>,
    },
}

/// A single-use delivery source constructed only by a validated durable read.
/// It reports persistence to the matching owning flow, never live authorization,
/// acknowledgement of consumption, or permission to release an executor slot.
/// ```compile_fail
/// use oteryn_gameplay_durability::durability::fresh_admission::DurableLossCompletionSource;
/// use oteryn_gameplay_foundation::foundation::ControlLossCompletionV1;
/// fn promote_history(history: ControlLossCompletionV1) -> DurableLossCompletionSource {
///     history.into()
/// }
/// ```
#[derive(Debug)]
pub struct DurableLossCompletionSource {
    completion: Option<Box<ControlLossCompletionV1>>,
    current: Box<FreshAdmissionDurableReconciliationSnapshotV1>,
}
impl DurableLossCompletionSource {
    #[must_use]
    pub fn current_snapshot(&self) -> &FreshAdmissionDurableReconciliationSnapshotV1 {
        &self.current
    }
}
impl crate::foundation::fnd04_verifier::recovery_source_sealed::Sealed
    for DurableLossCompletionSource
{
}
impl ControlLossCompletionSourceV1 for DurableLossCompletionSource {
    fn take_loss_completion(
        &mut self,
        original: &ControlLossOperationV1,
    ) -> std::result::Result<Option<ControlLossCompletionV1>, ReconnectDurabilityErrorV1> {
        if self
            .completion
            .as_ref()
            .is_some_and(|completion| &completion.operation != original)
        {
            // A wrong recipient does not consume or replace the real original.
            return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
        }
        Ok(self.completion.take().map(|completion| *completion))
    }
}

/// Asynchronous storage only. Production bounded scheduling/completion remains
/// a separate adapter requirement; runtime arguments must match fixed registry caps.
#[derive(Clone)]
pub struct FreshAdmissionStore {
    security_revocation_freshness_required: bool,
    retired_holder: Option<retired_holder_release::RetiredHolderProof>,
    // Rare Activity release proofs must not enlarge every ordinary admission
    // and reconnect future. Clones retain the same immutable, revalidated proof.
    departed_activity: Option<std::sync::Arc<super::activity_handoff::DepartedActivityRelease>>,
    staff_loss: Option<ControlLossEpochRefV1>,
    offline_activity_mismatch:
        Option<std::sync::Arc<super::activity_handoff::OfflineActivityMismatchRelease>>,
    guards: AdmissionGuardStore,
    maximum_operation_bytes: usize,
    /// CHAR-POSITION-0 §3.2 "At logout": the last position the terminal release writes in its
    /// own transaction, before `session_state` becomes 3. `None` writes nothing.
    final_position: Option<super::character_last_position::FinalLastPosition>,
    pending_vocation_refill: Option<super::character_build::BuildOccurrence>,
    /// Q16: named House metadata and the same current node proof used for its final capture.
    final_house: Option<(
        super::character_last_position::HouseLastPosition,
        super::runtime_scope_assignment::NodeIncarnationProof,
    )>,
    /// Original entry reservation cleanup shares the terminal commit even when no living capture exists.
    house_release_node: Option<super::runtime_scope_assignment::NodeIncarnationProof>,
    /// OFFLINE0: revalidated in the same physical terminal-release transaction.
    training_logout: Option<std::sync::Arc<dyn TerminalReleaseParticipant>>,
    journal_catalogue: Option<std::sync::Arc<super::journal::LogoutCatalogue>>,
    /// chsw: the release is the safe exit of a voluntary Channel switch; the Party membership is
    /// held for the fresh admission into the target Channel instead of removed.
    channel_switch: bool,
}

/// OFFLINE0: a terminal-release participant owned outside the shared core (LOOP-TRAIN-1's
/// logout marker), revalidated in the same physical transaction before `session_state` is 3.
pub trait TerminalReleaseParticipant: Send + Sync {
    fn prepare<'a, 'c>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>
    where
        'c: 'a;
}
mod portfolio;
pub(super) use portfolio::{lock_admission_portfolio, lock_admission_portfolios};

include!("fresh_admission/commit.rs");

include!("fresh_admission/control_loss.rs");

include!("fresh_admission/reconciliation.rs");

mod control_loss_codec;
use control_loss_codec::write_fresh_loss;
pub use control_loss_codec::{decode_fresh_loss, encode_fresh_loss};

include!("fresh_admission/session_use_ledger.rs");

/// A candidate-specific owner read, not a caller-provided membership answer.
pub struct DurableSessionUseSource {
    request: GameSessionUseRequestV1,
    observation: GameSessionUseObservationV1,
}
impl recovery_source_sealed::Sealed for DurableSessionUseSource {}
impl GameSessionUseObservationSourceV1 for DurableSessionUseSource {
    fn observe_candidate_use(
        &self,
        request: &GameSessionUseRequestV1,
    ) -> std::result::Result<GameSessionUseObservationV1, GameSessionUseAuthorizationErrorV1> {
        if request != &self.request {
            return Err(GameSessionUseAuthorizationErrorV1::StaleAuthority);
        }
        Ok(self.observation.clone())
    }
}

include!("fresh_admission/session_use_source.rs");

include!("fresh_admission/lifecycle.rs");

/// Outcome of one grace-expiry release attempt for a lost GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiredLossReleaseV1 {
    /// The session is not a reconnectable loss any more (resumed, replaced or
    /// already released); nothing is this attempt's to release.
    NotApplicable,
    /// The original grace deadline is still ahead on the durable clock.
    NotExpired { deadline: i64, now: i64 },
    /// The terminal release is durable (possibly by an earlier attempt).
    Released { decided_at: i64 },
    /// The lost session is already TERMINAL (an earlier attempt's commit whose
    /// acknowledgement was lost, or another terminal owner). The Channel actor
    /// bound to this exact session is no longer controllable.
    Terminal,
}

/// The owning release source for one expired loss: the current claim rows,
/// cleared for the terminal session at the durable decision time.
struct ExpiredLossReleaseOwner {
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    transition: AdmissionClaimTransitionEvidenceV1,
}
impl fresh_source_sealed::Sealed for ExpiredLossReleaseOwner {}
impl AdmissionClaimOwningSourceV1 for ExpiredLossReleaseOwner {
    fn prepare_lifecycle_claim(
        &self,
        operation: &AdmissionClaimLifecycleOperationV1,
        _now: i64,
    ) -> std::result::Result<
        AdmissionClaimLifecycleResolutionV1,
        AdmissionAuthorityPublicationErrorV1,
    > {
        if !matches!(
            operation,
            AdmissionClaimLifecycleOperationV1::TerminalRelease { .. }
        ) {
            return Err(AdmissionAuthorityPublicationErrorV1::Invalid);
        }
        Ok(AdmissionClaimLifecycleResolutionV1 {
            current_session: self.current,
            evidence: self.transition.clone(),
        })
    }
    fn prepare_fresh_claim(
        &self,
        _binding: &FreshAdmissionAuditBindingV1,
        _now: i64,
    ) -> std::result::Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>
    {
        Err(AdmissionAuthorityPublicationErrorV1::Unavailable)
    }
}

include!("fresh_admission/release_requests.rs");

include!("fresh_admission/replacement_claims.rs");

include!("fresh_admission/reconnect_codec.rs");

include!("fresh_admission/recovery_budget.rs");

include!("fresh_admission/reconnect_commit.rs");

include!("fresh_admission/reconnect_reconciliation.rs");

/// The owning reconnect source with its Recovery V2 evidence replaced by the registered
/// floors locked in the deciding transaction.
struct RegisteredRecoverySource<'a> {
    inner: &'a dyn CompleteReconnectSourceV1,
    registered: &'a dyn RecoveryDurabilityEvidenceSourceV2,
}
impl recovery_source_sealed::Sealed for RegisteredRecoverySource<'_> {}
impl CompleteReconnectSourceV1 for RegisteredRecoverySource<'_> {
    fn resolve_reconnect(
        &self,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> std::result::Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
        self.inner.resolve_reconnect(identity, now)
    }
    fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
        Some(self.registered)
    }
}

/// The retained budget as PREPARE leaves it: the historical budget with this attempt
/// added as `Prepared` (the durable reconstruction names no predecessor attempts here).
fn recovery_budget_after_prepare(
    recovery: &CompleteReconnectOperationV1,
) -> Result<RetainedRecoveryBudgetV1> {
    let original = &recovery.original.budget;
    let mut entries = original.entries().to_vec();
    if !entries
        .iter()
        .any(|entry| entry.attempt == recovery.identity.reconnect_attempt_ref())
    {
        entries.push(RetainedRecoveryAttemptV1 {
            attempt: recovery.identity.reconnect_attempt_ref(),
            transport: recovery.original.candidate.transport_ref(),
            disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
        });
    }
    checked(RetainedRecoveryBudgetV1::restore(
        original.epoch(),
        original.state(),
        true,
        entries,
    ))
}

#[cfg(test)]
mod resource_preflight_tests {
    use super::*;
    #[test]
    fn envelope_preflight_checks_exact_boundary_and_overflow() -> Result<()> {
        let exact = envelope_size(49_132, usize::MAX)?;
        assert_eq!(exact, super::super::MAX_FRESH_OPERATION_BYTES);
        assert!(envelope_size(49_133, exact).is_err());
        assert_eq!(envelope_size(49_132, exact)?, exact);
        assert!(envelope_size(49_132, exact - 1).is_err());
        assert!(envelope_size(usize::MAX, usize::MAX).is_err());
        let mut counter = Writer::counter(3);
        counter.bytes(&[1, 2, 3])?;
        assert_eq!(counter.measured(), 3);
        assert_eq!(counter.bytes.capacity(), 0);
        assert!(counter.bytes(&[4]).is_err());
        assert_eq!(counter.measured(), 3);
        assert_eq!(counter.bytes.capacity(), 0);
        Ok(())
    }
    #[test]
    fn runtime_caps_cannot_be_inflated_or_reconfigured() -> Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .block_on(async {
                for invalid in [
                    0,
                    super::super::MAX_FRESH_OPERATION_BYTES - 1,
                    super::super::MAX_FRESH_OPERATION_BYTES + 1,
                    usize::MAX,
                ] {
                    assert!(matches!(
                        FreshAdmissionStore::connect_runtime(
                            "not-a-database-url",
                            invalid,
                            super::super::MAX_ADMISSION_GUARD_BYTES
                        )
                        .await,
                        Err(DurabilityError::InvalidStoredState)
                    ));
                }
                for invalid in [
                    0,
                    super::super::MAX_ADMISSION_GUARD_BYTES - 1,
                    super::super::MAX_ADMISSION_GUARD_BYTES + 1,
                    usize::MAX,
                ] {
                    assert!(matches!(
                        AdmissionGuardStore::connect_runtime("not-a-database-url", invalid).await,
                        Err(DurabilityError::InvalidStoredState)
                    ));
                }
                Ok(())
            })
    }
}
