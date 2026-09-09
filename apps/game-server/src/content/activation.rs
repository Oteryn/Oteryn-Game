use super::model::ContentError;
use super::production::{
    DurableMigrationClass, FirstProductionExpectation, FirstProductionRuntimeState,
    GenerationIdentity, StagedGeneration,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

mod sealed {
    pub trait Sealed {}
}

/// External control-plane authorization consumed by the CONTENT activation boundary.
///
/// The sealing trait is crate-private, so downstream crates cannot mint an authorization
/// implementation merely because they can construct valid content bytes. A future accepted
/// Game authority lane may implement this trait inside this crate.
pub trait ContentActivationAdmissionGuard {
    fn quiescent(&self) -> bool;
    fn live_scope_count(&self) -> usize;
}

pub trait AuthorizedContentGeneration: sealed::Sealed {
    type AdmissionGuard<'a>: ContentActivationAdmissionGuard
    where
        Self: 'a;

    fn acquire_admission_guard(&self) -> Self::AdmissionGuard<'_>;
    fn target_identity(&self) -> &GenerationIdentity;
    fn expected_current_identity(&self) -> Option<&GenerationIdentity>;
    fn expected_current_activation_sequence(&self) -> Option<u64>;
    fn activation_sequence(&self) -> u64;
    fn permits_last_known_good_fallback(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentActivationError {
    Content(ContentError),
    CandidateAlreadyStaged,
    NoStagedCandidate,
    NotQuiescent,
    LiveScopesRemain(usize),
    AuthorizationTargetMismatch,
    ExpectedCurrentMismatch,
    NonMonotonicActivationSequence { attempted: u64, current_floor: u64 },
    MigrationBearingActivationRejected,
    LastKnownGoodFallbackNotAuthorized,
    LastKnownGoodReceiptMismatch,
}

impl Display for ContentActivationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(error) => write!(formatter, "{error}"),
            Self::CandidateAlreadyStaged => {
                formatter.write_str("a first-production content candidate is already staged")
            }
            Self::NoStagedCandidate => formatter.write_str("no content candidate is staged"),
            Self::NotQuiescent => {
                formatter.write_str("content activation requires an explicit quiescent boundary")
            }
            Self::LiveScopesRemain(count) => {
                write!(
                    formatter,
                    "content activation rejected while {count} live scopes remain"
                )
            }
            Self::AuthorizationTargetMismatch => {
                formatter.write_str("authorization target does not match the staged generation")
            }
            Self::ExpectedCurrentMismatch => {
                formatter.write_str("authorization expected-current generation identity or activation sequence is stale or wrong")
            }
            Self::NonMonotonicActivationSequence {
                attempted,
                current_floor,
            } => write!(
                formatter,
                "activation sequence must increase monotonically: {attempted} <= {current_floor}"
            ),
            Self::MigrationBearingActivationRejected => formatter.write_str(
                "migration-bearing content cannot activate in FIRST_PRODUCTION_CONTENT_PROFILE/v1",
            ),
            Self::LastKnownGoodFallbackNotAuthorized => formatter.write_str(
                "last-known-good fallback requires explicit external fallback authorization",
            ),
            Self::LastKnownGoodReceiptMismatch => formatter.write_str(
                "revalidated fallback bytes do not match the supplied last-known-good receipt",
            ),
        }
    }
}

impl Error for ContentActivationError {}

impl From<ContentError> for ContentActivationError {
    fn from(value: ContentError) -> Self {
        Self::Content(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveGeneration {
    identity: GenerationIdentity,
    activation_sequence: u64,
    runtime_state: FirstProductionRuntimeState,
}

impl ActiveGeneration {
    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub fn server_record_count(&self) -> usize {
        self.runtime_state.server_record_count()
    }

    pub fn client_record_count(&self) -> usize {
        self.runtime_state.client_record_count()
    }

    pub fn server_definition_fields(&self, key: &str) -> Option<&[String]> {
        self.runtime_state.server_definition_fields(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastKnownGoodReceipt {
    identity: GenerationIdentity,
    activation_sequence: u64,
}

impl LastKnownGoodReceipt {
    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub fn expectation(&self) -> FirstProductionExpectation {
        FirstProductionExpectation::from_generation_identity(&self.identity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StagedCandidateKind {
    Primary,
    LastKnownGoodFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StagedCandidate {
    generation: StagedGeneration,
    kind: StagedCandidateKind,
}

#[derive(Debug, Default)]
pub struct ContentActivationController {
    active: Option<ActiveGeneration>,
    staged: Option<StagedCandidate>,
    last_known_good: Option<LastKnownGoodReceipt>,
    last_activation_sequence: u64,
    ready: bool,
}

impl ContentActivationController {
    pub const fn new() -> Self {
        Self {
            active: None,
            staged: None,
            last_known_good: None,
            last_activation_sequence: 0,
            ready: false,
        }
    }

    /// Reconstruct the process-local activation controller after restart.
    ///
    /// Restart is deliberately not ready and does not restore an active generation. The receipt
    /// is immutable evidence only and raises the monotonic activation-sequence floor.
    pub fn restart_with_receipt(receipt: Option<LastKnownGoodReceipt>) -> Self {
        let last_activation_sequence = receipt
            .as_ref()
            .map_or(0, LastKnownGoodReceipt::activation_sequence);
        Self {
            active: None,
            staged: None,
            last_known_good: receipt,
            last_activation_sequence,
            ready: false,
        }
    }

    pub const fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn active(&self) -> Option<&ActiveGeneration> {
        self.active.as_ref()
    }

    pub fn staged_identity(&self) -> Option<&GenerationIdentity> {
        self.staged
            .as_ref()
            .map(|candidate| candidate.generation.identity())
    }

    pub fn last_known_good_receipt(&self) -> Option<&LastKnownGoodReceipt> {
        self.last_known_good.as_ref()
    }

    pub const fn last_activation_sequence(&self) -> u64 {
        self.last_activation_sequence
    }

    pub fn stage_primary(
        &mut self,
        server_bytes: &[u8],
        client_bytes: &[u8],
        expected: &FirstProductionExpectation,
    ) -> Result<&GenerationIdentity, ContentActivationError> {
        self.require_staging_slot()?;
        let generation = StagedGeneration::stage(server_bytes, client_bytes, expected)?;
        self.staged = Some(StagedCandidate {
            generation,
            kind: StagedCandidateKind::Primary,
        });
        self.staged_identity()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    pub fn stage_last_known_good_fallback(
        &mut self,
        server_bytes: &[u8],
        client_bytes: &[u8],
        expected: &FirstProductionExpectation,
        receipt: &LastKnownGoodReceipt,
    ) -> Result<&GenerationIdentity, ContentActivationError> {
        self.require_staging_slot()?;
        let generation = StagedGeneration::stage(server_bytes, client_bytes, expected)?;
        if generation.identity() != receipt.identity() {
            return Err(ContentActivationError::LastKnownGoodReceiptMismatch);
        }
        self.staged = Some(StagedCandidate {
            generation,
            kind: StagedCandidateKind::LastKnownGoodFallback,
        });
        self.staged_identity()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    pub fn discard_staged(&mut self) {
        self.staged = None;
    }

    pub fn activate<A: AuthorizedContentGeneration>(
        &mut self,
        authorization: &A,
    ) -> Result<&ActiveGeneration, ContentActivationError> {
        let candidate = self
            .staged
            .as_ref()
            .ok_or(ContentActivationError::NoStagedCandidate)?;

        // The externally owned guard must remain alive through every final authority check and
        // the non-fallible publication below. Its owning implementation is responsible for
        // holding new authoritative-scope admission while this value is alive.
        let admission_guard = authorization.acquire_admission_guard();
        if !admission_guard.quiescent() {
            return Err(ContentActivationError::NotQuiescent);
        }
        let live_scope_count = admission_guard.live_scope_count();
        if live_scope_count != 0 {
            return Err(ContentActivationError::LiveScopesRemain(live_scope_count));
        }
        if authorization.target_identity() != candidate.generation.identity() {
            return Err(ContentActivationError::AuthorizationTargetMismatch);
        }
        if !expected_current_matches(
            authorization.expected_current_identity(),
            authorization.expected_current_activation_sequence(),
            self.active.as_ref(),
        ) {
            return Err(ContentActivationError::ExpectedCurrentMismatch);
        }
        let activation_sequence = authorization.activation_sequence();
        if activation_sequence <= self.last_activation_sequence {
            return Err(ContentActivationError::NonMonotonicActivationSequence {
                attempted: activation_sequence,
                current_floor: self.last_activation_sequence,
            });
        }
        if candidate.generation.identity().migration_class()
            != DurableMigrationClass::CompatibleNoMigration
        {
            return Err(ContentActivationError::MigrationBearingActivationRejected);
        }
        if candidate.kind == StagedCandidateKind::LastKnownGoodFallback
            && !authorization.permits_last_known_good_fallback()
        {
            return Err(ContentActivationError::LastKnownGoodFallbackNotAuthorized);
        }

        // Exact commit point: all validation above is read-only. Only here is the complete staged
        // server+client generation removed from staging and published as the sole active pointer.
        let candidate = self
            .staged
            .take()
            .ok_or(ContentActivationError::NoStagedCandidate)?;
        let (identity, runtime_state) = candidate.generation.into_parts();
        let active = ActiveGeneration {
            identity: identity.clone(),
            activation_sequence,
            runtime_state,
        };
        self.active = Some(active);
        self.last_activation_sequence = activation_sequence;
        self.last_known_good = Some(LastKnownGoodReceipt {
            identity,
            activation_sequence,
        });
        self.ready = true;

        self.active
            .as_ref()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    fn require_staging_slot(&self) -> Result<(), ContentActivationError> {
        if self.staged.is_some() {
            return Err(ContentActivationError::CandidateAlreadyStaged);
        }
        Ok(())
    }
}

fn expected_current_matches(
    expected_identity: Option<&GenerationIdentity>,
    expected_sequence: Option<u64>,
    current: Option<&ActiveGeneration>,
) -> bool {
    match (expected_identity, expected_sequence, current) {
        (None, None, None) => true,
        (Some(expected_identity), Some(expected_sequence), Some(current)) => {
            expected_identity == current.identity()
                && expected_sequence == current.activation_sequence()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::production::{
        FirstProductionCompileTarget, compile_first_production, test_source,
    };
    use std::cell::Cell;

    #[derive(Debug, Clone)]
    struct TestAuthorization {
        target: GenerationIdentity,
        expected_current: Option<GenerationIdentity>,
        expected_current_sequence: Option<u64>,
        activation_sequence: u64,
        quiescent: bool,
        live_scopes: usize,
        permit_fallback: bool,
        hold_active: Cell<bool>,
        hold_observed_during_target_check: Cell<bool>,
    }

    struct TestAdmissionGuard<'a> {
        authorization: &'a TestAuthorization,
    }

    impl ContentActivationAdmissionGuard for TestAdmissionGuard<'_> {
        fn quiescent(&self) -> bool {
            self.authorization.quiescent
        }

        fn live_scope_count(&self) -> usize {
            self.authorization.live_scopes
        }
    }

    impl Drop for TestAdmissionGuard<'_> {
        fn drop(&mut self) {
            self.authorization.hold_active.set(false);
        }
    }

    impl sealed::Sealed for TestAuthorization {}

    impl AuthorizedContentGeneration for TestAuthorization {
        type AdmissionGuard<'a> = TestAdmissionGuard<'a>;

        fn acquire_admission_guard(&self) -> Self::AdmissionGuard<'_> {
            self.hold_active.set(true);
            TestAdmissionGuard {
                authorization: self,
            }
        }

        fn target_identity(&self) -> &GenerationIdentity {
            if self.hold_active.get() {
                self.hold_observed_during_target_check.set(true);
            }
            &self.target
        }

        fn expected_current_identity(&self) -> Option<&GenerationIdentity> {
            self.expected_current.as_ref()
        }

        fn expected_current_activation_sequence(&self) -> Option<u64> {
            self.expected_current_sequence
        }

        fn activation_sequence(&self) -> u64 {
            self.activation_sequence
        }

        fn permits_last_known_good_fallback(&self) -> bool {
            self.permit_fallback
        }
    }

    fn compiled(
        cell_count: usize,
    ) -> Result<crate::content::production::CompiledFirstProductionContent, ContentError> {
        let source = test_source(cell_count)?;
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)
    }

    fn authorization(
        target: &GenerationIdentity,
        expected_current: Option<(&GenerationIdentity, u64)>,
        sequence: u64,
    ) -> TestAuthorization {
        let (expected_current, expected_current_sequence) = match expected_current {
            Some((identity, sequence)) => (Some(identity.clone()), Some(sequence)),
            None => (None, None),
        };
        TestAuthorization {
            target: target.clone(),
            expected_current,
            expected_current_sequence,
            activation_sequence: sequence,
            quiescent: true,
            live_scopes: 0,
            permit_fallback: false,
            hold_active: Cell::new(false),
            hold_observed_during_target_check: Cell::new(false),
        }
    }

    #[test]
    fn restart_starts_not_ready_and_receipt_does_not_auto_activate() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let staged = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&staged, None, 1))?;
        let receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing receipt")?;

        let mut restarted =
            ContentActivationController::restart_with_receipt(Some(receipt.clone()));
        assert!(!restarted.is_ready());
        assert!(restarted.active().is_none());
        assert!(restarted.staged_identity().is_none());
        assert_eq!(restarted.last_activation_sequence(), 1);

        let restaged = restarted
            .stage_last_known_good_fallback(
                &compiled.server_artifact,
                &compiled.client_artifact,
                &receipt.expectation(),
                &receipt,
            )?
            .clone();
        assert_eq!(&restaged, receipt.identity());
        assert!(!restarted.is_ready());
        assert!(restarted.active().is_none());
        Ok(())
    }

    #[test]
    fn second_staged_candidate_is_denied_without_changing_first() -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();

        let result = controller.stage_primary(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
        );
        assert!(matches!(
            result,
            Err(ContentActivationError::CandidateAlreadyStaged)
        ));
        assert_eq!(controller.staged_identity(), Some(&first_identity));
        assert!(controller.active().is_none());
        Ok(())
    }

    #[test]
    fn activation_requires_quiescence_and_zero_live_scopes() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let identity = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();

        let mut auth = authorization(&identity, None, 1);
        auth.quiescent = false;
        assert!(matches!(
            controller.activate(&auth),
            Err(ContentActivationError::NotQuiescent)
        ));
        assert!(controller.active().is_none());
        assert_eq!(controller.staged_identity(), Some(&identity));

        auth.quiescent = true;
        auth.live_scopes = 1;
        assert!(matches!(
            controller.activate(&auth),
            Err(ContentActivationError::LiveScopesRemain(1))
        ));
        assert!(controller.active().is_none());
        assert_eq!(controller.staged_identity(), Some(&identity));
        Ok(())
    }

    #[test]
    fn stale_current_wrong_target_and_nonmonotonic_sequence_fail_precommit()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 10))?;
        let active_before = controller.active().cloned().ok_or("active missing")?;

        controller.stage_primary(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
        )?;
        let second_identity = controller
            .staged_identity()
            .cloned()
            .ok_or("staged missing")?;

        let wrong_target = authorization(&first_identity, Some((&first_identity, 10)), 11);
        assert!(matches!(
            controller.activate(&wrong_target),
            Err(ContentActivationError::AuthorizationTargetMismatch)
        ));
        assert_eq!(controller.active(), Some(&active_before));

        let stale_expected = authorization(&second_identity, None, 11);
        assert!(matches!(
            controller.activate(&stale_expected),
            Err(ContentActivationError::ExpectedCurrentMismatch)
        ));
        assert_eq!(controller.active(), Some(&active_before));

        let stale_sequence = authorization(&second_identity, Some((&first_identity, 9)), 11);
        assert!(matches!(
            controller.activate(&stale_sequence),
            Err(ContentActivationError::ExpectedCurrentMismatch)
        ));
        assert_eq!(controller.active(), Some(&active_before));

        let nonmonotonic = authorization(&second_identity, Some((&first_identity, 10)), 10);
        assert!(matches!(
            controller.activate(&nonmonotonic),
            Err(ContentActivationError::NonMonotonicActivationSequence { .. })
        ));
        assert_eq!(controller.active(), Some(&active_before));
        assert_eq!(controller.staged_identity(), Some(&second_identity));
        Ok(())
    }

    #[test]
    fn commit_publishes_one_complete_generation_only() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let identity = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();

        let auth = authorization(&identity, None, 1);
        let active = controller.activate(&auth)?;
        assert_eq!(active.identity(), &identity);
        assert_eq!(active.activation_sequence(), 1);
        assert_eq!(active.server_record_count(), 22);
        assert_eq!(active.client_record_count(), 6);
        assert_eq!(
            active
                .server_definition_fields("oteryn:prod.creature")
                .and_then(|fields| fields.get(3))
                .map(String::as_str),
            Some("creature-policy-r1")
        );
        assert!(auth.hold_observed_during_target_check.get());
        assert!(!auth.hold_active.get());
        assert!(controller.staged_identity().is_none());
        assert!(controller.is_ready());
        assert_eq!(
            controller
                .last_known_good_receipt()
                .map(LastKnownGoodReceipt::identity),
            Some(&identity)
        );
        Ok(())
    }

    #[test]
    fn explicitly_authorized_last_known_good_fallback_revalidates_bytes_and_sequence()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 1))?;
        let old_receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing first receipt")?;

        let second_identity = controller
            .stage_primary(
                &second.server_artifact,
                &second.client_artifact,
                second.expectation(),
            )?
            .clone();
        controller.activate(&authorization(
            &second_identity,
            Some((&first_identity, 1)),
            2,
        ))?;

        controller.stage_last_known_good_fallback(
            &first.server_artifact,
            &first.client_artifact,
            first.expectation(),
            &old_receipt,
        )?;
        let mut fallback = authorization(&first_identity, Some((&second_identity, 2)), 3);
        assert!(matches!(
            controller.activate(&fallback),
            Err(ContentActivationError::LastKnownGoodFallbackNotAuthorized)
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&second_identity)
        );

        fallback.permit_fallback = true;
        let rolled_back = controller.activate(&fallback)?;
        assert_eq!(rolled_back.identity(), &first_identity);
        assert_eq!(rolled_back.activation_sequence(), 3);
        Ok(())
    }

    #[test]
    fn fallback_receipt_mismatch_and_corrupt_primary_fail_before_publication()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 1))?;
        let receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing receipt")?;
        controller.discard_staged();

        let mismatch = controller.stage_last_known_good_fallback(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
            &receipt,
        );
        assert!(matches!(
            mismatch,
            Err(ContentActivationError::LastKnownGoodReceiptMismatch)
        ));
        assert!(controller.staged_identity().is_none());

        let mut corrupt = first.server_artifact.clone();
        let index = corrupt.len() - 33;
        corrupt[index] ^= 0x5a;
        let corrupt_result =
            controller.stage_primary(&corrupt, &first.client_artifact, first.expectation());
        assert!(matches!(
            corrupt_result,
            Err(ContentActivationError::Content(
                ContentError::IntegrityMismatch(_)
            ))
        ));
        assert!(controller.staged_identity().is_none());
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&first_identity)
        );
        Ok(())
    }
}
