//! Split-phase fresh admission. Historical data never registers a current source.
//! Ports enqueue bounded work; database waits belong outside the FND-03 writer.
use super::admission_authority_publication::*;
use super::fnd04_verifier::{
    FreshAccountSecurityObservationV1, FreshCurrentEvidence, FreshEvidenceProvenanceV1,
    FreshSigningTrustObservationV1, VerifiedFreshDurabilityFactsV1,
};
use super::{
    AuthenticatedTransportRefV1, FreshAdmissionCommit, FreshAdmissionFacts,
    GameSessionAuthoritySnapshot, GameSessionId, PRE_ADMISSION_PROFILE, RuntimeScopeRefV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshAdmissionDurabilityErrorV1 {
    Invalid,
    StaleAuthority,
    WrongBinding,
    WrongPhase,
    Unavailable,
}

/// Persistable, non-secret historical DTO. Constructing/restoring this value
/// confers no live authorization, source registration or controller authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshAdmissionAuditBindingV1 {
    pub version: u8,
    pub account_id: String,
    pub facts: FreshAdmissionFacts,
    pub candidate_session: GameSessionId,
    pub transport: AuthenticatedTransportRefV1,
    pub connection_generation: u64,
    pub current_facts: FreshCurrentEvidence,
    pub protocol_major: u64,
    pub transport_profile: u64,
    pub signed_security_generation: u64,
    pub signing: FreshSigningTrustObservationV1,
    pub security: FreshAccountSecurityObservationV1,
    pub credential_times: (i64, i64, i64),
    pub verified_at: i64,
    pub accepted_deadline: i64,
    pub expected_guards: Vec<AdmissionAuthorityPublicationChangeV1>,
}
impl FreshAdmissionAuditBindingV1 {
    pub fn initial_commit(
        &self,
    ) -> Result<FreshAdmissionCommit<AuthenticatedTransportRefV1>, FreshAdmissionDurabilityErrorV1>
    {
        if self.version != 1 || self.connection_generation != 1 {
            return Err(FreshAdmissionDurabilityErrorV1::Invalid);
        }
        FreshAdmissionCommit::from_facts(self.candidate_session, self.facts, self.transport)
            .map_err(|_| FreshAdmissionDurabilityErrorV1::Invalid)
    }
    /// Structural historical validation only, not credential authentication.
    pub fn validate_historical(&self) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        use super::fnd04_verifier::{FreshEvidencePurposeV1, NumericDate, fresh_source_deadline};
        let fail = FreshAdmissionDurabilityErrorV1::Invalid;
        let commit = self.initial_commit()?;
        let f = &self.current_facts;
        let (iat, nbf, exp) = self.credential_times;
        NumericDate::validate(self.verified_at, iat, nbf, exp).map_err(|_| fail)?;
        let signing_deadline = fresh_source_deadline(
            &self.signing.provenance,
            FreshEvidencePurposeV1::SigningTrust,
            self.verified_at,
        )
        .map_err(|_| fail)?;
        let security_deadline = fresh_source_deadline(
            &self.security.provenance,
            FreshEvidencePurposeV1::PlatformSecurity,
            self.verified_at,
        )
        .map_err(|_| fail)?;
        let deadline = exp
            .checked_add(4)
            .ok_or(fail)?
            .min(iat.checked_add(35).ok_or(fail)?)
            .min(signing_deadline)
            .min(security_deadline);
        if self.accepted_deadline != deadline
            || self.verified_at > deadline
            || self.expected_guards.len() != 4
            || self.account_id != f.account_id
            || self.protocol_major == 0
            || self.transport_profile == 0
            || self.signed_security_generation == 0
            || !self.signing.trusted
            || !self.security.allowed
            || self.security.account_id != self.account_id
            || commit.character_id() != f.character_id
            || commit.world_id() != f.world_id
            || commit.channel_id() != f.channel_id
            || commit.scope_ownership_generation() != f.scope_ownership_generation
            || f.character_lease_generation.checked_add(1)
                != Some(commit.character_lease_generation())
        {
            return Err(fail);
        }
        let rows: Vec<_> = self.expected_guards.iter().cloned().map(Some).collect();
        validate_current_guards(self, &rows, self.verified_at, false).map_err(|_| fail)
    }
    fn keys(&self) -> Vec<AdmissionAuthorityGuardKeyV1> {
        vec![
            AdmissionAuthorityGuardKeyV1::Account {
                account_id: self.account_id.clone(),
            },
            AdmissionAuthorityGuardKeyV1::Character(self.current_facts.character_id),
            AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(
                self.current_facts.world_id,
                self.current_facts.channel_id,
            )),
            AdmissionAuthorityGuardKeyV1::SigningTrust {
                key_id: self.signing.key_id.clone(),
                profile: PRE_ADMISSION_PROFILE.into(),
            },
        ]
    }
}

/// Only verified claims and independently current published guards can create
/// the live capability. A historical DTO intentionally cannot be converted.
/// ```compile_fail
/// use oteryn_game_server::foundation::fresh_admission_durability::*;
/// fn forge(binding: FreshAdmissionAuditBindingV1) -> FreshAdmissionCommitAuthorizationV1 {
///     FreshAdmissionCommitAuthorizationV1::from(binding)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FreshAdmissionCommitAuthorizationV1 {
    binding: FreshAdmissionAuditBindingV1,
}
impl FreshAdmissionCommitAuthorizationV1 {
    pub fn new(
        facts: &VerifiedFreshDurabilityFactsV1,
        candidate_session: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        current: &dyn AdmissionAuthorityPublicationCurrentSourceV1,
        now: i64,
    ) -> Result<Self, FreshAdmissionDurabilityErrorV1> {
        if now != facts.verified_at() || now > facts.accepted_deadline() {
            return Err(FreshAdmissionDurabilityErrorV1::StaleAuthority);
        }
        let (protocol_major, transport_profile) = facts.protocol_transport();
        let mut binding = FreshAdmissionAuditBindingV1 {
            version: 1,
            account_id: facts.account_id().into(),
            facts: *facts.facts(),
            candidate_session,
            transport,
            connection_generation: 1,
            current_facts: facts.current().facts.clone(),
            protocol_major,
            transport_profile,
            signed_security_generation: facts.signed_security_generation(),
            signing: facts.signing().clone(),
            security: facts.security().clone(),
            credential_times: facts.credential_times(),
            verified_at: now,
            accepted_deadline: facts.accepted_deadline(),
            expected_guards: vec![],
        };
        let rows = current
            .current_publications(&binding.keys())
            .map_err(|_| FreshAdmissionDurabilityErrorV1::Unavailable)?;
        let revisions = [
            facts.current().account_publication_revision,
            facts.current().character_publication_revision,
            facts.current().runtime_publication_revision,
            facts.signing().provenance.publication_revision,
        ];
        validate_current_guards(&binding, &rows, now, false)?;
        for (row, revision) in rows.into_iter().zip(revisions) {
            let row = row.ok_or(FreshAdmissionDurabilityErrorV1::StaleAuthority)?;
            if row.publication_revision != revision {
                return Err(FreshAdmissionDurabilityErrorV1::StaleAuthority);
            }
            binding.expected_guards.push(row);
        }
        Ok(Self { binding })
    }
    /// Pure final-L predicate over independently locked current rows. The adapter
    /// must acquire every potentially blocking claim/reservation before sampling
    /// trusted time, then hold all guards through conditional atomic COMMIT.
    pub fn validate_at_decision(
        &self,
        rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
        trusted_now: Option<i64>,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        let fail = FreshAdmissionDurabilityErrorV1::StaleAuthority;
        let now = trusted_now.ok_or(fail)?;
        if now < self.binding.verified_at
            || now > self.binding.accepted_deadline
            || rows.len() != 4
            || rows
                .iter()
                .zip(&self.binding.expected_guards)
                .any(|(row, expected)| row.as_ref() != Some(expected))
        {
            return Err(fail);
        }
        validate_current_guards(&self.binding, rows, now, false)
    }
    #[must_use]
    pub const fn binding(&self) -> &FreshAdmissionAuditBindingV1 {
        &self.binding
    }
}

/// Complete immutable identity used for durable replay and recovery. This is
/// historical data, not a live authorization or owning-source registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshAdmissionOperationV1 {
    pub authorization: FreshAdmissionAuditBindingV1,
    pub transition: AdmissionClaimTransitionEvidenceV1,
}
impl FreshAdmissionOperationV1 {
    pub fn validate_historical(
        &self,
        decided_at: i64,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        self.authorization.validate_historical()?;
        validate_fresh_claim_evidence(&self.authorization, &self.transition, decided_at)
            .map_err(|_| FreshAdmissionDurabilityErrorV1::Invalid)
    }
}

#[derive(Debug, Clone)]
pub struct FreshAdmissionCommitRequestV1 {
    authorization: FreshAdmissionCommitAuthorizationV1,
    transition: FreshAdmissionClaimTransitionV1,
    operation: FreshAdmissionOperationV1,
}
impl FreshAdmissionCommitRequestV1 {
    pub fn validate_at_decision(
        &self,
        rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
        trusted_now: Option<i64>,
    ) -> Result<&[AdmissionAuthorityPublicationChangeV1], FreshAdmissionDurabilityErrorV1> {
        self.authorization.validate_at_decision(rows, trusted_now)?;
        self.transition
            .validate_locked(
                rows,
                trusted_now.ok_or(FreshAdmissionDurabilityErrorV1::StaleAuthority)?,
            )
            .map_err(|_| FreshAdmissionDurabilityErrorV1::StaleAuthority)?;
        Ok(&self.transition.evidence().successors)
    }
    #[must_use]
    pub const fn binding(&self) -> &FreshAdmissionAuditBindingV1 {
        self.authorization.binding()
    }
    #[must_use]
    pub const fn operation(&self) -> &FreshAdmissionOperationV1 {
        &self.operation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshAdmissionCommitReceiptV1 {
    operation: FreshAdmissionOperationV1,
    decided_at: i64,
}
impl FreshAdmissionCommitReceiptV1 {
    /// Restore historical committed evidence at original L; never a submit capability.
    pub fn restore(
        operation: FreshAdmissionOperationV1,
        decided_at: i64,
    ) -> Result<Self, FreshAdmissionDurabilityErrorV1> {
        operation.validate_historical(decided_at)?;
        Ok(Self {
            operation,
            decided_at,
        })
    }
    #[must_use]
    pub const fn binding(&self) -> &FreshAdmissionAuditBindingV1 {
        &self.operation.authorization
    }
    #[must_use]
    pub const fn operation(&self) -> &FreshAdmissionOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn decided_at(&self) -> i64 {
        self.decided_at
    }
    pub fn classify_retry(
        &self,
        operation: &FreshAdmissionOperationV1,
    ) -> FreshAdmissionDurableOutcomeV1 {
        if &self.operation == operation {
            FreshAdmissionDurableOutcomeV1::ExistingCommitted(self.clone())
        } else {
            FreshAdmissionDurableOutcomeV1::RejectedReplayConflict
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshAdmissionSubmissionV1 {
    Accepted,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshAdmissionCollisionV1 {
    CandidateSession,
    TransportReference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshAdmissionDurableOutcomeV1 {
    Committed(FreshAdmissionCommitReceiptV1),
    ExistingCommitted(FreshAdmissionCommitReceiptV1),
    RejectedReplayConflict,
    RejectedIncumbent,
    RejectedStaleAuthority,
    /// Proven noncommit; the original candidate/reference cannot be reused.
    RejectedCollision(FreshAdmissionCollisionV1),
    AmbiguousOrUnavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshAdmissionDurableReconciliationSnapshotV1 {
    pub receipt: FreshAdmissionCommitReceiptV1,
    /// Adapter must read this with the receipt at one fenced durable read.
    pub current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
}
pub trait FreshAdmissionDurabilityPortV1 {
    /// Try one bounded enqueue, never wait for I/O or completion in the writer.
    fn submit(&mut self, request: &FreshAdmissionCommitRequestV1) -> FreshAdmissionSubmissionV1;
    /// Read/reconcile only the original replay/candidate/transport binding.
    fn reconcile(&mut self, original: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1;
}
/// An owning current source, not a receipt-to-current conversion. Child C binds
/// real producers; missing source/floor/physical transport mapping stays closed.
pub trait FreshAdmissionCurrentSourceV1: AdmissionAuthorityPublicationCurrentSourceV1 {
    /// A restored durable reference is not a new physical connection. Default
    /// absence is closed; producers prove the original authenticated mapping.
    fn has_live_transport(&self, _transport: AuthenticatedTransportRefV1) -> bool {
        false
    }
    fn current_session(
        &self,
        session: GameSessionId,
    ) -> Result<
        GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        FreshAdmissionDurabilityErrorV1,
    >;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshAdmissionPhaseV1 {
    Ready,
    PendingCommit,
    ReconciliationRequired,
    PendingReconciliation,
    AwaitingAdoption,
    Adopted,
    Rejected,
}

#[derive(Debug)]
pub struct FreshAdmissionDurabilityFlowV1 {
    request: Option<FreshAdmissionCommitRequestV1>,
    binding: FreshAdmissionAuditBindingV1,
    operation: FreshAdmissionOperationV1,
    phase: FreshAdmissionPhaseV1,
    receipt: Option<FreshAdmissionCommitReceiptV1>,
    reconciled_session: Option<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>>,
    projection: super::admission::FreshAdmissionProjectionV1,
}
impl FreshAdmissionDurabilityFlowV1 {
    /// ```compile_fail
    /// use oteryn_game_server::foundation::fresh_admission_durability::*;
    /// fn unpaired(auth: FreshAdmissionCommitAuthorizationV1) {
    ///     FreshAdmissionDurabilityFlowV1::begin(auth);
    /// }
    /// ```
    pub fn begin(
        authorization: FreshAdmissionCommitAuthorizationV1,
        transition: FreshAdmissionClaimTransitionV1,
    ) -> Result<Self, FreshAdmissionDurabilityErrorV1> {
        if authorization.binding() != transition.binding() {
            return Err(FreshAdmissionDurabilityErrorV1::WrongBinding);
        }
        let operation = FreshAdmissionOperationV1 {
            authorization: authorization.binding().clone(),
            transition: transition.evidence().clone(),
        };
        Ok(Self {
            binding: authorization.binding.clone(),
            operation: operation.clone(),
            request: Some(FreshAdmissionCommitRequestV1 {
                authorization,
                transition,
                operation,
            }),
            phase: FreshAdmissionPhaseV1::Ready,
            receipt: None,
            reconciled_session: None,
            projection: super::admission::FreshAdmissionProjectionV1::default(),
        })
    }
    /// Restart can reconcile historical data, but cannot submit a new commit.
    #[must_use]
    pub fn resume_reconciliation(operation: FreshAdmissionOperationV1) -> Self {
        Self {
            request: None,
            binding: operation.authorization.clone(),
            operation,
            phase: FreshAdmissionPhaseV1::ReconciliationRequired,
            receipt: None,
            reconciled_session: None,
            projection: super::admission::FreshAdmissionProjectionV1::default(),
        }
    }
    #[must_use]
    pub const fn operation(&self) -> &FreshAdmissionOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn binding(&self) -> &FreshAdmissionAuditBindingV1 {
        &self.binding
    }
    #[must_use]
    pub const fn phase(&self) -> FreshAdmissionPhaseV1 {
        self.phase
    }
    #[must_use]
    pub const fn controller(&self) -> Option<FreshAdmissionCommit<AuthenticatedTransportRefV1>> {
        self.projection.controller()
    }
    #[must_use]
    pub const fn receipt(&self) -> Option<&FreshAdmissionCommitReceiptV1> {
        self.receipt.as_ref()
    }
    pub fn submit(
        &mut self,
        port: &mut dyn FreshAdmissionDurabilityPortV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::Ready {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        let request = self
            .request
            .as_ref()
            .ok_or(FreshAdmissionDurabilityErrorV1::WrongPhase)?;
        self.accept_submission(port.submit(request))
    }
    pub fn accept_submission(
        &mut self,
        result: FreshAdmissionSubmissionV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::Ready {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        if result == FreshAdmissionSubmissionV1::Accepted {
            self.phase = FreshAdmissionPhaseV1::PendingCommit;
        }
        Ok(())
    }
    pub fn accept_completion(
        &mut self,
        operation: FreshAdmissionOperationV1,
        outcome: FreshAdmissionDurableOutcomeV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::PendingCommit {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        if operation != self.operation {
            return Err(FreshAdmissionDurabilityErrorV1::WrongBinding);
        }
        match outcome {
            FreshAdmissionDurableOutcomeV1::Committed(receipt)
            | FreshAdmissionDurableOutcomeV1::ExistingCommitted(receipt) => {
                self.accept_receipt(receipt)?
            }
            FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable => {
                self.phase = FreshAdmissionPhaseV1::ReconciliationRequired
            }
            _ => self.phase = FreshAdmissionPhaseV1::Rejected,
        }
        Ok(())
    }
    fn accept_receipt(
        &mut self,
        receipt: FreshAdmissionCommitReceiptV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if receipt.operation != self.operation {
            return Err(FreshAdmissionDurabilityErrorV1::WrongBinding);
        }
        self.receipt = Some(receipt);
        self.phase = FreshAdmissionPhaseV1::AwaitingAdoption;
        Ok(())
    }
    pub fn reconcile(
        &mut self,
        port: &mut dyn FreshAdmissionDurabilityPortV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::ReconciliationRequired {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        if port.reconcile(&self.operation) == FreshAdmissionSubmissionV1::Accepted {
            self.phase = FreshAdmissionPhaseV1::PendingReconciliation;
        }
        Ok(())
    }
    /// A missing/uncertain fenced read is not proof of abort. Yield and retain
    /// the same immutable binding for another bounded reconciliation request.
    pub fn accept_reconciliation_unavailable(
        &mut self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::PendingReconciliation {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        if original != &self.operation {
            return Err(FreshAdmissionDurabilityErrorV1::WrongBinding);
        }
        self.phase = FreshAdmissionPhaseV1::ReconciliationRequired;
        Ok(())
    }
    pub fn accept_reconciliation(
        &mut self,
        snapshot: FreshAdmissionDurableReconciliationSnapshotV1,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        if self.phase != FreshAdmissionPhaseV1::PendingReconciliation {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        if snapshot.current_session.commit() != self.binding.initial_commit()? {
            return Err(FreshAdmissionDurabilityErrorV1::WrongBinding);
        }
        self.accept_receipt(snapshot.receipt)?;
        self.reconciled_session = Some(snapshot.current_session);
        Ok(())
    }
    pub fn adopt(
        &mut self,
        source: &dyn FreshAdmissionCurrentSourceV1,
        now: i64,
    ) -> Result<(), FreshAdmissionDurabilityErrorV1> {
        self.projection.clear();
        if !matches!(
            self.phase,
            FreshAdmissionPhaseV1::AwaitingAdoption | FreshAdmissionPhaseV1::Adopted
        ) {
            return Err(FreshAdmissionDurabilityErrorV1::WrongPhase);
        }
        self.phase = FreshAdmissionPhaseV1::AwaitingAdoption;
        let fail = FreshAdmissionDurabilityErrorV1::StaleAuthority;
        if self
            .receipt
            .as_ref()
            .is_none_or(|receipt| now < receipt.decided_at)
        {
            return Err(fail);
        }
        let rows = source
            .current_publications(&self.binding.keys())
            .map_err(|_| fail)?;
        validate_current_guards(&self.binding, &rows, now, true)?;
        for (row, committed) in rows.iter().zip(&self.operation.transition.successors) {
            let current = row.as_ref().ok_or(fail)?;
            if current.publication_revision < committed.publication_revision
                || current.source.source_revision < committed.source.source_revision
                || current.source.source_observed_at < committed.source.source_observed_at
                || ((current.publication_revision == committed.publication_revision
                    || current.source.source_revision == committed.source.source_revision)
                    && current != committed)
            {
                return Err(fail);
            }
        }
        if !source.has_live_transport(self.binding.transport) {
            return Err(fail);
        }
        let current = source.current_session(self.binding.candidate_session)?;
        if self
            .reconciled_session
            .is_some_and(|snapshot| snapshot != current)
        {
            return Err(fail);
        }
        self.projection
            .install_current(self.binding.initial_commit()?, current)
            .map_err(|_| fail)?;
        self.phase = FreshAdmissionPhaseV1::Adopted;
        Ok(())
    }
}

fn validate_current_guards(
    binding: &FreshAdmissionAuditBindingV1,
    rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
    now: i64,
    acquired: bool,
) -> Result<(), FreshAdmissionDurabilityErrorV1> {
    use AdmissionAuthorityGuardStateV1 as State;
    let fail = FreshAdmissionDurabilityErrorV1::StaleAuthority;
    let keys = binding.keys();
    let f = &binding.current_facts;
    if rows.len() != 4 {
        return Err(fail);
    }
    for (index, (row, key)) in rows.iter().zip(keys).enumerate() {
        let row = row.as_ref().ok_or(fail)?;
        if row.key != key {
            return Err(fail);
        }
        validate_change(row, now).map_err(|_| fail)?;
        if let Some(old) = binding.expected_guards.get(index)
            && (((row.publication_revision == old.publication_revision
                || row.source.source_revision == old.source.source_revision)
                && row != old)
                || row.publication_revision < old.publication_revision
                || row.source.authority != old.source.authority
                || row.source.purpose != old.source.purpose
                || row.source.source_revision < old.source.source_revision
                || row.source.source_observed_at < old.source.source_observed_at
                || (row.source.source_revision == old.source.source_revision
                    && row.source != old.source))
        {
            return Err(fail);
        }
        let valid = match &row.state {
            State::Account { security, presence } => {
                let expected = if acquired {
                    Some((f.character_id, binding.candidate_session))
                } else {
                    None
                };
                security.account_id == binding.account_id
                    && security.allowed
                    && security.minimum_generation <= binding.signed_security_generation
                    && security.minimum_generation >= binding.security.minimum_generation
                    && (!acquired && security == &binding.security
                        || acquired && security_decision_consistent(security, &binding.security))
                    && source_not_rolled_back(&security.provenance, &binding.security.provenance)
                    && *presence == expected
            }
            State::Character {
                account_id,
                world_id,
                eligible,
                lease_generation,
                holder,
            } => {
                account_id == &binding.account_id
                    && *world_id == f.world_id
                    && *eligible
                    && *lease_generation
                        == if acquired {
                            binding.initial_commit()?.character_lease_generation()
                        } else {
                            f.character_lease_generation
                        }
                    && *holder
                        == if acquired {
                            Some(binding.candidate_session)
                        } else {
                            None
                        }
            }
            State::Runtime {
                ownership_generation,
                ready,
                route_revision,
                runtime_observation_revision,
                protocol_major,
                transport_profile,
                ruleset_revision,
                content_revision,
                map_revision,
                world_policy_revision,
                offer_revision,
            } => {
                *ownership_generation == f.scope_ownership_generation
                    && *ready
                    && route_revision == &f.route_revision
                    && runtime_observation_revision == &f.runtime_observation_revision
                    && *protocol_major == binding.protocol_major
                    && *transport_profile == binding.transport_profile
                    && ruleset_revision == &f.ruleset_revision
                    && content_revision == &f.content_revision
                    && map_revision == &f.map_revision
                    && world_policy_revision == &f.world_policy_revision
                    && offer_revision == &f.offer_revision
            }
            State::SigningTrust {
                public_key,
                trusted,
            } => {
                *trusted
                    && *public_key == binding.signing.public_key
                    && row.source.authority == binding.signing.provenance.source_authority
                    && (acquired
                        || row.source.source_revision == binding.signing.provenance.source_revision)
                    && row.source.source_revision >= binding.signing.provenance.source_revision
                    && row.source.source_observed_at
                        >= binding.signing.provenance.source_observed_at
                    && (row.source.source_revision != binding.signing.provenance.source_revision
                        || (row.source.decision_identity
                            == binding.signing.provenance.decision_identity
                            && row.source.source_observed_at
                                == binding.signing.provenance.source_observed_at
                            && row.source.clock_uncertainty_seconds
                                == binding.signing.provenance.clock_uncertainty_seconds))
            }
        };
        if !valid {
            return Err(fail);
        }
    }
    Ok(())
}
fn source_not_rolled_back(
    current: &FreshEvidenceProvenanceV1,
    old: &FreshEvidenceProvenanceV1,
) -> bool {
    current.source_authority == old.source_authority
        && current.purpose == old.purpose
        && current.scope == old.scope
        && current.source_revision >= old.source_revision
        && current.source_observed_at >= old.source_observed_at
        && (current.source_revision != old.source_revision
            || (current.decision_identity == old.decision_identity
                && current.source_observed_at == old.source_observed_at
                && current.clock_uncertainty_seconds == old.clock_uncertainty_seconds))
}

fn security_decision_consistent(
    current: &FreshAccountSecurityObservationV1,
    old: &FreshAccountSecurityObservationV1,
) -> bool {
    if current.provenance.source_revision != old.provenance.source_revision {
        return true;
    }
    let mut historical = old.clone();
    historical.provenance.publication_revision = current.provenance.publication_revision;
    &historical == current
}
