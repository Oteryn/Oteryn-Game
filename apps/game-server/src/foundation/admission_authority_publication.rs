//! Owning-source publication is preparation, never active Game authority.
//! The adapter must apply every change atomically under fresh-admission guards.
use super::fnd04_verifier::{
    Fnd04EvidenceScope, FreshAccountSecurityObservationV1, FreshEvidenceProvenanceV1,
    FreshEvidencePurposeV1, fresh_source_sealed,
};
use super::{
    AuthenticatedTransportRefV1, CharacterId, GameSessionAuthoritySnapshot, GameSessionId,
    GameSessionState, ReconnectDurabilityRecordV1, RuntimeScopeRefV1,
    TerminalGameSessionReplacementAuthorizationV1, WorldId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionAuthorityGuardKeyV1 {
    Account { account_id: String },
    Character(CharacterId),
    Runtime(RuntimeScopeRefV1),
    SigningTrust { key_id: String, profile: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionAuthorityGuardStateV1 {
    Account {
        security: FreshAccountSecurityObservationV1,
        presence: Option<(CharacterId, GameSessionId)>,
    },
    Character {
        account_id: String,
        world_id: WorldId,
        eligible: bool,
        lease_generation: u64,
        holder: Option<GameSessionId>,
    },
    Runtime {
        ownership_generation: u64,
        ready: bool,
        route_revision: String,
        runtime_observation_revision: String,
        protocol_major: u64,
        transport_profile: u64,
        ruleset_revision: String,
        content_revision: String,
        map_revision: String,
        world_policy_revision: String,
        offer_revision: String,
    },
    SigningTrust {
        public_key: [u8; 32],
        trusted: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionPublicationPurposeV1 {
    AccountSecurityAndPresence,
    CharacterOwnershipAndLease,
    RuntimeOwnershipAndReadiness,
    FixedFreshSigningTrust,
}

/// Raw owning-source observation, not a registration capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPublicationSourceV1 {
    pub authority: String,
    pub purpose: AdmissionPublicationPurposeV1,
    pub source_revision: u64,
    pub decision_identity: String,
    pub source_observed_at: i64,
    pub clock_uncertainty_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionPublicationPreconditionV1 {
    /// Independently restored absence/high-water evidence is mandatory.
    Bootstrap {
        restored_publication_high_water: Option<u64>,
    },
    CompareAndSet {
        expected_publication_revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityPublicationChangeV1 {
    pub key: AdmissionAuthorityGuardKeyV1,
    pub source: AdmissionPublicationSourceV1,
    pub precondition: AdmissionPublicationPreconditionV1,
    pub publication_revision: u64,
    pub state: AdmissionAuthorityGuardStateV1,
}

/// Only an independently authenticated owning adapter inside this crate can
/// implement the sealing supertrait. Raw fields and receipts cannot register it.
///
/// ```compile_fail
/// use oteryn_game_server::foundation::admission_authority_publication::*;
/// struct Caller;
/// impl AdmissionAuthorityOwningPublisherV1 for Caller {
///     fn resolve_publication(&self, _: i64) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1> {
///         Ok(Vec::new())
///     }
/// }
/// ```
pub trait AdmissionAuthorityOwningPublisherV1: fresh_source_sealed::Sealed {
    fn resolve_publication(
        &self,
        now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionAuthorityPublicationErrorV1 {
    Unavailable,
    Invalid,
    Stale,
    Conflict,
    WrongPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityPublicationV1 {
    changes: Vec<AdmissionAuthorityPublicationChangeV1>,
}

impl AdmissionAuthorityPublicationV1 {
    pub fn prepare(
        source: &dyn AdmissionAuthorityOwningPublisherV1,
        now: i64,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        let changes = source.resolve_publication(now)?;
        if changes.is_empty() {
            return Err(AdmissionAuthorityPublicationErrorV1::Invalid);
        }
        for (index, change) in changes.iter().enumerate() {
            if changes[..index].iter().any(|prior| prior.key == change.key) {
                return Err(AdmissionAuthorityPublicationErrorV1::Conflict);
            }
            validate_change(change, now)?;
        }
        Ok(Self { changes })
    }

    #[must_use]
    pub fn changes(&self) -> &[AdmissionAuthorityPublicationChangeV1] {
        &self.changes
    }

    /// Pure comparison over adapter-provided locked durable rows. This neither
    /// writes rows nor activates a producer. All rows must be checked atomically.
    pub fn validate_locked(
        &self,
        current: &[Option<AdmissionAuthorityPublicationChangeV1>],
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        use AdmissionAuthorityPublicationErrorV1::{Conflict, Stale};
        if current.len() != self.changes.len() {
            return Err(Conflict);
        }
        for (change, row) in self.changes.iter().zip(current) {
            if row.as_ref() == Some(change) {
                continue;
            } // Exact replay never re-ages evidence.
            match (change.precondition, row) {
                (
                    AdmissionPublicationPreconditionV1::Bootstrap {
                        restored_publication_high_water: Some(0),
                    },
                    None,
                ) => {
                    if has_claim(&change.state) {
                        return Err(Conflict);
                    }
                }
                (
                    AdmissionPublicationPreconditionV1::CompareAndSet {
                        expected_publication_revision,
                    },
                    Some(prior),
                ) => {
                    if prior.key != change.key
                        || prior.source.authority != change.source.authority
                        || prior.source.purpose != change.source.purpose
                    {
                        return Err(Conflict);
                    }
                    if prior.publication_revision != expected_publication_revision
                        || change.source.source_revision < prior.source.source_revision
                        || change.source.source_observed_at < prior.source.source_observed_at
                    {
                        return Err(Stale);
                    }
                    if change.source.source_revision == prior.source.source_revision {
                        return Err(Conflict);
                    }
                    match (&prior.state, &change.state) {
                        (
                            AdmissionAuthorityGuardStateV1::Account {
                                security: previous, ..
                            },
                            AdmissionAuthorityGuardStateV1::Account { security: next, .. },
                        ) if next.provenance.source_authority
                            != previous.provenance.source_authority
                            || next.provenance.source_observed_at
                                < previous.provenance.source_observed_at
                            || next.minimum_generation < previous.minimum_generation
                            || next.provenance.source_revision
                                < previous.provenance.source_revision
                            || (next.provenance.source_revision
                                == previous.provenance.source_revision
                                && !same_security_observation(next, previous)) =>
                        {
                            return Err(Stale);
                        }
                        (
                            AdmissionAuthorityGuardStateV1::Character {
                                lease_generation: old,
                                ..
                            },
                            AdmissionAuthorityGuardStateV1::Character {
                                lease_generation: new,
                                ..
                            },
                        ) if new < old => return Err(Stale),
                        (
                            AdmissionAuthorityGuardStateV1::Runtime {
                                ownership_generation: old,
                                ..
                            },
                            AdmissionAuthorityGuardStateV1::Runtime {
                                ownership_generation: new,
                                ..
                            },
                        ) if new < old => return Err(Stale),
                        _ => {}
                    }
                    if !same_claims(&prior.state, &change.state) {
                        return Err(Conflict);
                    }
                }
                _ => return Err(Stale),
            }
        }
        Ok(())
    }
}

fn has_claim(state: &AdmissionAuthorityGuardStateV1) -> bool {
    match state {
        AdmissionAuthorityGuardStateV1::Account { presence, .. } => presence.is_some(),
        AdmissionAuthorityGuardStateV1::Character { holder, .. } => holder.is_some(),
        _ => false,
    }
}

fn same_claims(
    left: &AdmissionAuthorityGuardStateV1,
    right: &AdmissionAuthorityGuardStateV1,
) -> bool {
    match (left, right) {
        (
            AdmissionAuthorityGuardStateV1::Account { presence: a, .. },
            AdmissionAuthorityGuardStateV1::Account { presence: b, .. },
        ) => a == b,
        (
            AdmissionAuthorityGuardStateV1::Character {
                holder: a,
                lease_generation: ag,
                account_id: aa,
                world_id: aw,
                ..
            },
            AdmissionAuthorityGuardStateV1::Character {
                holder: b,
                lease_generation: bg,
                account_id: ba,
                world_id: bw,
                ..
            },
        ) => a == b && ag == bg && aa == ba && aw == bw,
        (
            AdmissionAuthorityGuardStateV1::Runtime { .. },
            AdmissionAuthorityGuardStateV1::Runtime { .. },
        )
        | (
            AdmissionAuthorityGuardStateV1::SigningTrust { .. },
            AdmissionAuthorityGuardStateV1::SigningTrust { .. },
        ) => true,
        _ => false,
    }
}

fn same_security_observation(
    left: &FreshAccountSecurityObservationV1,
    right: &FreshAccountSecurityObservationV1,
) -> bool {
    let mut accepted = left.clone();
    // This wrapper binds the unchanged Platform observation to the newly
    // committed Account guard; it is not source time or source revision.
    accepted.provenance.publication_revision = right.provenance.publication_revision;
    accepted == *right
}

fn valid_security_provenance(provenance: &FreshEvidenceProvenanceV1, now: i64) -> bool {
    provenance.purpose == FreshEvidencePurposeV1::PlatformSecurity
        && provenance.scope == Fnd04EvidenceScope::FreshAdmission
        && !provenance.source_authority.is_empty()
        && provenance.source_revision > 0
        && provenance.source_revision == provenance.accepted_source_revision
        && !provenance.decision_identity.is_empty()
        && provenance.decision_identity == provenance.accepted_decision_identity
        && provenance.publication_revision > 0
        && source_age_valid(
            provenance.source_observed_at,
            provenance.clock_uncertainty_seconds,
            now,
        )
}

fn source_age_valid(observed: i64, uncertainty: u64, now: i64) -> bool {
    observed >= 0
        && now >= observed
        && i64::try_from(uncertainty)
            .ok()
            .and_then(|uncertainty| now.checked_sub(observed)?.checked_add(uncertainty))
            .is_some_and(|age| age <= 5)
}

pub(super) fn validate_change(
    change: &AdmissionAuthorityPublicationChangeV1,
    now: i64,
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    use AdmissionAuthorityGuardKeyV1 as Key;
    use AdmissionAuthorityGuardStateV1 as State;
    use AdmissionPublicationPurposeV1 as Purpose;
    let valid = match (&change.key, &change.state, change.source.purpose) {
        (
            Key::Account { account_id },
            State::Account { security, .. },
            Purpose::AccountSecurityAndPresence,
        ) => {
            !account_id.is_empty()
                && security.account_id == *account_id
                && security.minimum_generation > 0
                && security.provenance.publication_revision == change.publication_revision
                && valid_security_provenance(&security.provenance, now)
        }
        (
            Key::Character(_),
            State::Character {
                account_id,
                lease_generation,
                ..
            },
            Purpose::CharacterOwnershipAndLease,
        ) => !account_id.is_empty() && *lease_generation > 0,
        (
            Key::Runtime(_),
            State::Runtime {
                ownership_generation,
                route_revision,
                runtime_observation_revision,
                protocol_major,
                transport_profile,
                ruleset_revision,
                content_revision,
                map_revision,
                world_policy_revision,
                offer_revision,
                ..
            },
            Purpose::RuntimeOwnershipAndReadiness,
        ) => {
            *ownership_generation > 0
                && *protocol_major > 0
                && *transport_profile > 0
                && [
                    route_revision,
                    runtime_observation_revision,
                    ruleset_revision,
                    content_revision,
                    map_revision,
                    world_policy_revision,
                    offer_revision,
                ]
                .iter()
                .all(|revision| !revision.is_empty())
        }
        (
            Key::SigningTrust { key_id, profile },
            State::SigningTrust { .. },
            Purpose::FixedFreshSigningTrust,
        ) => !key_id.is_empty() && profile == "oteryn-pre-admission-v1",
        _ => false,
    };
    let expected = match change.precondition {
        AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        } => Some(1),
        AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision,
        } if expected_publication_revision > 0 => expected_publication_revision.checked_add(1),
        _ => None,
    };
    let freshness = !matches!(
        change.source.purpose,
        Purpose::AccountSecurityAndPresence | Purpose::FixedFreshSigningTrust
    ) || source_age_valid(
        change.source.source_observed_at,
        change.source.clock_uncertainty_seconds,
        now,
    );
    if !valid
        || !freshness
        || change.source.authority.is_empty()
        || change.source.source_revision == 0
        || change.source.decision_identity.is_empty()
        || change.source.source_observed_at < 0
        || change.source.source_observed_at > now
        || expected != Some(change.publication_revision)
    {
        return Err(AdmissionAuthorityPublicationErrorV1::Invalid);
    }
    Ok(())
}

/// Lossless historical conditional effects. Public data is not an owning capability.
/// The adapter must atomically enforce source/decision high-water marks across
/// transactions: these stateless predicates cannot prove global decision uniqueness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionClaimTransitionEvidenceV1 {
    pub predecessors: Vec<AdmissionAuthorityPublicationChangeV1>,
    pub successors: Vec<AdmissionAuthorityPublicationChangeV1>,
    pub prepared_at: i64,
}

/// Registration belongs to the owning Game adapter, never to an audit/receipt consumer.
/// ```compile_fail
/// use oteryn_game_server::foundation::admission_authority_publication::*;
/// struct Unregistered;
/// impl AdmissionClaimOwningSourceV1 for Unregistered {
///     fn prepare_fresh_claim(&self, _: &oteryn_game_server::foundation::fresh_admission_durability::FreshAdmissionAuditBindingV1, _: i64)
///         -> Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1> {
///         Err(AdmissionAuthorityPublicationErrorV1::Unavailable)
///     }
/// }
/// ```
pub trait AdmissionClaimOwningSourceV1: fresh_source_sealed::Sealed {
    fn prepare_lifecycle_claim(
        &self,
        _operation: &AdmissionClaimLifecycleOperationV1,
        _now: i64,
    ) -> Result<AdmissionClaimLifecycleResolutionV1, AdmissionAuthorityPublicationErrorV1> {
        Err(AdmissionAuthorityPublicationErrorV1::Unavailable)
    }
    fn prepare_fresh_claim(
        &self,
        binding: &super::fresh_admission_durability::FreshAdmissionAuditBindingV1,
        now: i64,
    ) -> Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>;
}

/// Prepared source changes remain inert until the matching atomic session COMMIT.
/// ```compile_fail
/// use oteryn_game_server::foundation::admission_authority_publication::*;
/// fn forge(history: AdmissionClaimTransitionEvidenceV1) -> FreshAdmissionClaimTransitionV1 {
///     FreshAdmissionClaimTransitionV1::from(history)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FreshAdmissionClaimTransitionV1 {
    binding: super::fresh_admission_durability::FreshAdmissionAuditBindingV1,
    evidence: AdmissionClaimTransitionEvidenceV1,
}
impl FreshAdmissionClaimTransitionV1 {
    pub fn prepare(
        owner: &dyn AdmissionClaimOwningSourceV1,
        authorization: &super::fresh_admission_durability::FreshAdmissionCommitAuthorizationV1,
        now: i64,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        let binding = authorization.binding();
        let evidence = owner.prepare_fresh_claim(binding, now)?;
        let result = Self {
            binding: binding.clone(),
            evidence,
        };
        // The owner supplied independent predecessors; compare them to the
        // authorization here. Actual locked current rows are required at L.
        validate_fresh_claim_evidence(binding, &result.evidence, now)?;
        Ok(result)
    }
    #[must_use]
    pub const fn binding(
        &self,
    ) -> &super::fresh_admission_durability::FreshAdmissionAuditBindingV1 {
        &self.binding
    }
    #[must_use]
    pub const fn evidence(&self) -> &AdmissionClaimTransitionEvidenceV1 {
        &self.evidence
    }
    pub fn validate_locked(
        &self,
        rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
        now: i64,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        validate_fresh_claim_evidence(&self.binding, &self.evidence, now)?;
        if rows.len() != 4
            || rows
                .iter()
                .zip(&self.binding.expected_guards)
                .any(|(row, prior)| row.as_ref() != Some(prior))
        {
            return Err(AdmissionAuthorityPublicationErrorV1::Stale);
        }
        Ok(())
    }
}

/// Exact intended canonical session effect, not authorization to apply it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionClaimLifecycleOperationV1 {
    TerminalRelease {
        account_id: String,
        current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    },
    TerminalReplacement {
        account_id: String,
        current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        candidate: Box<ReconnectDurabilityRecordV1>,
    },
}
impl AdmissionClaimLifecycleOperationV1 {
    #[must_use]
    pub fn account_id(&self) -> &str {
        match self {
            Self::TerminalRelease { account_id, .. }
            | Self::TerminalReplacement { account_id, .. } => account_id,
        }
    }
    #[must_use]
    pub const fn current_session(
        &self,
    ) -> GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1> {
        match self {
            Self::TerminalRelease {
                current_session, ..
            }
            | Self::TerminalReplacement {
                current_session, ..
            } => *current_session,
        }
    }
}

/// The registered owner independently resolves both session and claim predecessors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionClaimLifecycleResolutionV1 {
    pub current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub evidence: AdmissionClaimTransitionEvidenceV1,
}

/// Persistable historical operation/effects; no conversion into a live capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionClaimLifecycleEvidenceV1 {
    pub operation: AdmissionClaimLifecycleOperationV1,
    pub transition: AdmissionClaimTransitionEvidenceV1,
}
impl AdmissionClaimLifecycleEvidenceV1 {
    pub fn validate_historical(
        &self,
        decided_at: i64,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        validate_lifecycle_effects(&self.operation, &self.transition, decided_at)
    }
}

/// Only a registered owner can prepare this inert release, and it must accompany
/// the exact fenced canonical terminal session write. A receipt is not authority.
/// ```compile_fail
/// use oteryn_game_server::foundation::admission_authority_publication::*;
/// fn forge(history: AdmissionClaimLifecycleEvidenceV1) -> TerminalReleaseClaimTransitionV1 {
///     TerminalReleaseClaimTransitionV1::from(history)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TerminalReleaseClaimTransitionV1 {
    evidence: AdmissionClaimLifecycleEvidenceV1,
}
impl TerminalReleaseClaimTransitionV1 {
    pub fn prepare(
        owner: &dyn AdmissionClaimOwningSourceV1,
        account_id: &str,
        current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        let operation = AdmissionClaimLifecycleOperationV1::TerminalRelease {
            account_id: account_id.into(),
            current_session,
        };
        Ok(Self {
            evidence: prepare_lifecycle(owner, operation, now)?,
        })
    }
    #[must_use]
    pub const fn evidence(&self) -> &AdmissionClaimLifecycleEvidenceV1 {
        &self.evidence
    }
    pub fn validate_locked(
        &self,
        rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
    ) -> Result<&[AdmissionAuthorityPublicationChangeV1], AdmissionAuthorityPublicationErrorV1>
    {
        validate_lifecycle_locked(&self.evidence, rows, current, now)?;
        Ok(&self.evidence.transition.successors)
    }
}

/// Inert owner-authored replacement; canonical reconnect V1/V2 authorization
/// and all candidate/session effects remain the durable adapter's prerequisites.
/// ```compile_fail
/// use oteryn_game_server::foundation::admission_authority_publication::*;
/// fn forge(history: AdmissionClaimLifecycleEvidenceV1) -> TerminalReplacementClaimTransitionV1 {
///     TerminalReplacementClaimTransitionV1::from(history)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TerminalReplacementClaimTransitionV1 {
    evidence: AdmissionClaimLifecycleEvidenceV1,
}
impl TerminalReplacementClaimTransitionV1 {
    pub fn prepare(
        owner: &dyn AdmissionClaimOwningSourceV1,
        authorization: &TerminalGameSessionReplacementAuthorizationV1,
        current_session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        candidate: &ReconnectDurabilityRecordV1,
        now: i64,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        let presence = super::AccountPresenceClaimV1::new(
            authorization.account_id(),
            current_session.commit().character_id(),
        )
        .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let expected = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            authorization.account_id(),
            Some(&presence),
            current_session.commit().game_session_id(),
            candidate.identity().game_session_id(),
            current_session,
            candidate,
        )
        .map_err(|_| AdmissionAuthorityPublicationErrorV1::Stale)?;
        if &expected != authorization {
            return Err(AdmissionAuthorityPublicationErrorV1::Stale);
        }
        let operation = AdmissionClaimLifecycleOperationV1::TerminalReplacement {
            account_id: authorization.account_id().into(),
            current_session,
            candidate: Box::new(candidate.clone()),
        };
        Ok(Self {
            evidence: prepare_lifecycle(owner, operation, now)?,
        })
    }
    #[must_use]
    pub const fn evidence(&self) -> &AdmissionClaimLifecycleEvidenceV1 {
        &self.evidence
    }
    pub fn validate_locked(
        &self,
        rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        candidate: &ReconnectDurabilityRecordV1,
        now: i64,
    ) -> Result<&[AdmissionAuthorityPublicationChangeV1], AdmissionAuthorityPublicationErrorV1>
    {
        if !matches!(&self.evidence.operation, AdmissionClaimLifecycleOperationV1::TerminalReplacement { candidate: expected, .. } if expected.as_ref() == candidate)
        {
            return Err(AdmissionAuthorityPublicationErrorV1::Conflict);
        }
        validate_lifecycle_locked(&self.evidence, rows, current, now)?;
        Ok(&self.evidence.transition.successors)
    }
}

fn prepare_lifecycle(
    owner: &dyn AdmissionClaimOwningSourceV1,
    operation: AdmissionClaimLifecycleOperationV1,
    now: i64,
) -> Result<AdmissionClaimLifecycleEvidenceV1, AdmissionAuthorityPublicationErrorV1> {
    let resolution = owner.prepare_lifecycle_claim(&operation, now)?;
    if resolution.current_session != operation.current_session() {
        return Err(AdmissionAuthorityPublicationErrorV1::Stale);
    }
    let evidence = AdmissionClaimLifecycleEvidenceV1 {
        operation,
        transition: resolution.evidence,
    };
    evidence.validate_historical(now)?;
    Ok(evidence)
}

fn validate_lifecycle_locked(
    evidence: &AdmissionClaimLifecycleEvidenceV1,
    rows: &[Option<AdmissionAuthorityPublicationChangeV1>],
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    now: i64,
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    if current != evidence.operation.current_session()
        || rows.len() != 2
        || rows
            .iter()
            .zip(&evidence.transition.predecessors)
            .any(|(row, prior)| row.as_ref() != Some(prior))
    {
        return Err(AdmissionAuthorityPublicationErrorV1::Stale);
    }
    evidence.validate_historical(now)
}

fn validate_session_claims(
    account_id: &str,
    snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    claims: &[AdmissionAuthorityPublicationChangeV1],
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    use AdmissionAuthorityPublicationErrorV1::Stale;
    let commit = snapshot.commit();
    let lease = snapshot.current_character_lease();
    if claims.len() != 2
        || account_id.is_empty()
        || lease.character_id() != commit.character_id()
        || lease.generation() < commit.character_lease_generation()
        || snapshot.current_connection_generation().get() < commit.connection_generation().get()
        || snapshot.current_scope_generation().get() < commit.scope_ownership_generation()
        || snapshot.current_runtime_scope().world_id() != commit.world_id()
        || !matches!(
            (snapshot.session_state(), snapshot.current_transport()),
            (GameSessionState::Active, Some(_))
                | (
                    GameSessionState::Reconnectable | GameSessionState::Terminal,
                    None
                )
        )
    {
        return Err(Stale);
    }
    match (
        &claims[0].key,
        &claims[0].state,
        &claims[1].key,
        &claims[1].state,
    ) {
        (
            AdmissionAuthorityGuardKeyV1::Account { account_id: a },
            AdmissionAuthorityGuardStateV1::Account {
                presence: Some((character, session)),
                ..
            },
            AdmissionAuthorityGuardKeyV1::Character(c),
            AdmissionAuthorityGuardStateV1::Character {
                account_id: b,
                world_id,
                lease_generation,
                holder: Some(holder),
                ..
            },
        ) if a == account_id
            && b == account_id
            && *character == commit.character_id()
            && *c == *character
            && *session == commit.game_session_id()
            && *holder == *session
            && *world_id == commit.world_id()
            && *lease_generation == lease.generation() =>
        {
            Ok(())
        }
        _ => Err(Stale),
    }
}

/// Additional predicate for an already authorized reconnect/control-loss write.
/// It cannot grant that write: both snapshots and both complete claim rows must
/// be independently locked, and all holder/lease/source values are preserved.
pub fn validate_claim_preserving_session_v1(
    account_id: &str,
    expected: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    expected_claims: &[AdmissionAuthorityPublicationChangeV1],
    current_claims: &[Option<AdmissionAuthorityPublicationChangeV1>],
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    if current != expected
        || current_claims.len() != 2
        || expected_claims.len() != 2
        || current_claims
            .iter()
            .zip(expected_claims)
            .any(|(current, expected)| current.as_ref() != Some(expected))
    {
        return Err(AdmissionAuthorityPublicationErrorV1::Stale);
    }
    validate_session_claims(account_id, current, expected_claims)
}

fn validate_lifecycle_effects(
    operation: &AdmissionClaimLifecycleOperationV1,
    evidence: &AdmissionClaimTransitionEvidenceV1,
    now: i64,
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    use AdmissionAuthorityPublicationErrorV1::Invalid;
    validate_claim_pair(evidence, now)?;
    validate_session_claims(
        operation.account_id(),
        operation.current_session(),
        &evidence.predecessors,
    )?;
    match operation {
        AdmissionClaimLifecycleOperationV1::TerminalRelease { .. } => {
            match (
                &evidence.successors[0].state,
                &evidence.predecessors[1].state,
                &evidence.successors[1].state,
            ) {
                (
                    AdmissionAuthorityGuardStateV1::Account { presence: None, .. },
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation: old,
                        ..
                    },
                    AdmissionAuthorityGuardStateV1::Character {
                        holder: None,
                        lease_generation: new,
                        ..
                    },
                ) if old == new => Ok(()),
                _ => Err(Invalid),
            }
        }
        AdmissionClaimLifecycleOperationV1::TerminalReplacement {
            account_id,
            current_session,
            candidate,
        } => {
            let presence = super::AccountPresenceClaimV1::new(
                account_id,
                current_session.commit().character_id(),
            )
            .map_err(|_| Invalid)?;
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                account_id,
                Some(&presence),
                current_session.commit().game_session_id(),
                candidate.identity().game_session_id(),
                *current_session,
                candidate,
            )
            .map_err(|_| Invalid)?;
            let identity = candidate.identity();
            if now > candidate.continuity().prepared_deadline()
                || now > candidate.continuity().original_grace_deadline()
            {
                return Err(Invalid);
            }
            match (
                &evidence.successors[0].state,
                &evidence.predecessors[1].state,
                &evidence.successors[1].state,
            ) {
                (
                    AdmissionAuthorityGuardStateV1::Account {
                        presence: Some((character, session)),
                        ..
                    },
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation: old,
                        ..
                    },
                    AdmissionAuthorityGuardStateV1::Character {
                        holder: Some(holder),
                        lease_generation: new,
                        ..
                    },
                ) if *character == identity.character_id()
                    && *session == identity.game_session_id()
                    && *holder == *session
                    && old == new
                    && *new == candidate.authority().character_lease_generation() =>
                {
                    Ok(())
                }
                _ => Err(Invalid),
            }
        }
    }
}

pub(super) fn validate_fresh_claim_evidence(
    binding: &super::fresh_admission_durability::FreshAdmissionAuditBindingV1,
    evidence: &AdmissionClaimTransitionEvidenceV1,
    now: i64,
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    use AdmissionAuthorityPublicationErrorV1::Invalid;
    binding.validate_historical().map_err(|_| Invalid)?;
    if evidence.prepared_at < binding.verified_at
        || now > binding.accepted_deadline
        || now < evidence.prepared_at
        || evidence.predecessors.as_slice() != &binding.expected_guards[..2]
    {
        return Err(Invalid);
    }
    validate_claim_pair(evidence, now)?;
    let commit = binding.initial_commit().map_err(|_| Invalid)?;
    let expected_presence = Some((commit.character_id(), commit.game_session_id()));
    match (
        &evidence.predecessors[0].state,
        &evidence.successors[0].state,
        &evidence.predecessors[1].state,
        &evidence.successors[1].state,
    ) {
        (
            AdmissionAuthorityGuardStateV1::Account { presence: None, .. },
            AdmissionAuthorityGuardStateV1::Account { presence, .. },
            AdmissionAuthorityGuardStateV1::Character {
                holder: None,
                lease_generation: old,
                ..
            },
            AdmissionAuthorityGuardStateV1::Character {
                holder,
                lease_generation,
                ..
            },
        ) if *presence == expected_presence
            && *holder == Some(commit.game_session_id())
            && old.checked_add(1) == Some(*lease_generation)
            && *lease_generation == commit.character_lease_generation() =>
        {
            Ok(())
        }
        _ => Err(Invalid),
    }
}

fn validate_claim_pair(
    evidence: &AdmissionClaimTransitionEvidenceV1,
    now: i64,
) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
    use AdmissionAuthorityPublicationErrorV1::{Invalid, Stale};
    if evidence.predecessors.len() != 2
        || evidence.successors.len() != 2
        || evidence.prepared_at < 0
        || now < evidence.prepared_at
    {
        return Err(Invalid);
    }
    for (prior, next) in evidence.predecessors.iter().zip(&evidence.successors) {
        validate_change(prior, now)?;
        validate_change(next, now)?;
        if prior.key != next.key
            || prior.source.authority != next.source.authority
            || prior.source.purpose != next.source.purpose
            || next.precondition
                != (AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: prior.publication_revision,
                })
            || prior.publication_revision.checked_add(1) != Some(next.publication_revision)
            || next.source.source_revision <= prior.source.source_revision
            || next.source.decision_identity == prior.source.decision_identity
            || next.source.source_observed_at < prior.source.source_observed_at
            || next.source.source_observed_at != evidence.prepared_at
        {
            return Err(Stale);
        }
    }
    match (
        &evidence.predecessors[0].key,
        &evidence.predecessors[0].state,
        &evidence.successors[0].state,
        &evidence.predecessors[1].key,
        &evidence.predecessors[1].state,
        &evidence.successors[1].state,
    ) {
        (
            AdmissionAuthorityGuardKeyV1::Account { account_id },
            AdmissionAuthorityGuardStateV1::Account {
                security: before, ..
            },
            AdmissionAuthorityGuardStateV1::Account {
                security: after, ..
            },
            AdmissionAuthorityGuardKeyV1::Character(_),
            AdmissionAuthorityGuardStateV1::Character {
                account_id: a,
                world_id: w,
                eligible: e,
                ..
            },
            AdmissionAuthorityGuardStateV1::Character {
                account_id: b,
                world_id: x,
                eligible: f,
                ..
            },
        ) if same_security_observation(before, after)
            && a == b
            && a == account_id
            && w == x
            && e == f =>
        {
            Ok(())
        }
        _ => Err(Invalid),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionAuthorityPublicationSubmissionV1 {
    Accepted,
    Unavailable,
}

/// A bounded queue interface. Implementations enqueue and return; no database
/// or network operation may hold the logical writer.
pub trait AdmissionAuthorityPublicationPortV1 {
    fn submit(
        &mut self,
        request: &AdmissionAuthorityPublicationV1,
    ) -> AdmissionAuthorityPublicationSubmissionV1;
    fn reconcile(
        &mut self,
        request: &AdmissionAuthorityPublicationV1,
    ) -> AdmissionAuthorityPublicationSubmissionV1;
}

/// Privileged durable adapter's already-normalized completion, not an I/O API.
/// Its independent response is not reconstructed from a caller's request.
pub trait AdmissionAuthorityPublicationCompletionSourceV1: fresh_source_sealed::Sealed {
    fn accepted_publication(
        &self,
    ) -> Result<AdmissionAuthorityPublicationV1, AdmissionAuthorityPublicationErrorV1>;
}

/// Independently current owner projection at activation, resolved as another
/// normalized input. Missing/rollback-uncertain state remains closed.
pub trait AdmissionAuthorityPublicationCurrentSourceV1: fresh_source_sealed::Sealed {
    fn current_publications(
        &self,
        keys: &[AdmissionAuthorityGuardKeyV1],
    ) -> Result<
        Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
        AdmissionAuthorityPublicationErrorV1,
    >;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionAuthorityPublicationReceiptV1 {
    accepted: AdmissionAuthorityPublicationV1,
}
impl AdmissionAuthorityPublicationReceiptV1 {
    pub fn resolve(
        source: &dyn AdmissionAuthorityPublicationCompletionSourceV1,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        Ok(Self {
            accepted: source.accepted_publication()?,
        })
    }
    #[must_use]
    pub const fn accepted(&self) -> &AdmissionAuthorityPublicationV1 {
        &self.accepted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionAuthorityPublicationPhaseV1 {
    Ready,
    Pending,
    ReconcileRequired,
    Active,
}

#[derive(Debug)]
pub struct AdmissionAuthorityPublicationFlowV1 {
    request: AdmissionAuthorityPublicationV1,
    phase: AdmissionAuthorityPublicationPhaseV1,
}
impl AdmissionAuthorityPublicationFlowV1 {
    #[must_use]
    pub const fn new(request: AdmissionAuthorityPublicationV1) -> Self {
        Self {
            request,
            phase: AdmissionAuthorityPublicationPhaseV1::Ready,
        }
    }
    #[must_use]
    pub const fn phase(&self) -> AdmissionAuthorityPublicationPhaseV1 {
        self.phase
    }
    pub fn submit(
        &mut self,
        port: &mut dyn AdmissionAuthorityPublicationPortV1,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        if self.phase != AdmissionAuthorityPublicationPhaseV1::Ready {
            return Err(AdmissionAuthorityPublicationErrorV1::WrongPhase);
        }
        if port.submit(&self.request) == AdmissionAuthorityPublicationSubmissionV1::Unavailable {
            return Err(AdmissionAuthorityPublicationErrorV1::Unavailable);
        }
        self.phase = AdmissionAuthorityPublicationPhaseV1::Pending;
        Ok(())
    }
    pub fn ambiguous(&mut self) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        if self.phase != AdmissionAuthorityPublicationPhaseV1::Pending {
            return Err(AdmissionAuthorityPublicationErrorV1::WrongPhase);
        }
        self.phase = AdmissionAuthorityPublicationPhaseV1::ReconcileRequired;
        Ok(())
    }
    pub fn reconcile(
        &mut self,
        port: &mut dyn AdmissionAuthorityPublicationPortV1,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        if self.phase != AdmissionAuthorityPublicationPhaseV1::ReconcileRequired {
            return Err(AdmissionAuthorityPublicationErrorV1::WrongPhase);
        }
        if port.reconcile(&self.request) == AdmissionAuthorityPublicationSubmissionV1::Unavailable {
            return Err(AdmissionAuthorityPublicationErrorV1::Unavailable);
        }
        Ok(())
    }
    pub fn accept(
        &mut self,
        receipt: &AdmissionAuthorityPublicationReceiptV1,
        current: &dyn AdmissionAuthorityPublicationCurrentSourceV1,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        if !matches!(
            self.phase,
            AdmissionAuthorityPublicationPhaseV1::Pending
                | AdmissionAuthorityPublicationPhaseV1::ReconcileRequired
        ) {
            return Err(AdmissionAuthorityPublicationErrorV1::WrongPhase);
        }
        if receipt.accepted != self.request {
            return Err(AdmissionAuthorityPublicationErrorV1::Conflict);
        }
        let keys: Vec<_> = self
            .request
            .changes
            .iter()
            .map(|change| change.key.clone())
            .collect();
        let independently_current = current.current_publications(&keys)?;
        if independently_current.len() != self.request.changes.len()
            || independently_current
                .iter()
                .zip(&self.request.changes)
                .any(|(row, expected)| row.as_ref() != Some(expected))
        {
            return Err(AdmissionAuthorityPublicationErrorV1::Stale);
        }
        self.phase = AdmissionAuthorityPublicationPhaseV1::Active;
        Ok(())
    }
    #[must_use]
    pub fn active_publication(&self) -> Option<&AdmissionAuthorityPublicationV1> {
        (self.phase == AdmissionAuthorityPublicationPhaseV1::Active).then_some(&self.request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_security() -> FreshEvidenceProvenanceV1 {
        FreshEvidenceProvenanceV1 {
            source_authority: "authenticated-platform-security".into(),
            purpose: FreshEvidencePurposeV1::PlatformSecurity,
            scope: Fnd04EvidenceScope::FreshAdmission,
            source_revision: 7,
            accepted_source_revision: 7,
            decision_identity: "security-seven".into(),
            accepted_decision_identity: "security-seven".into(),
            source_observed_at: 100,
            clock_uncertainty_seconds: 0,
            publication_revision: 1,
        }
    }
    struct Owner;
    impl fresh_source_sealed::Sealed for Owner {}
    impl AdmissionAuthorityOwningPublisherV1 for Owner {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(vec![AdmissionAuthorityPublicationChangeV1 {
                key: AdmissionAuthorityGuardKeyV1::Account {
                    account_id: "00000000-0000-4000-8000-000000000001".into(),
                },
                source: AdmissionPublicationSourceV1 {
                    authority: "authenticated-owner".into(),
                    purpose: AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
                    source_revision: 7,
                    decision_identity: "decision-seven".into(),
                    source_observed_at: 100,
                    clock_uncertainty_seconds: 0,
                },
                precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                    restored_publication_high_water: Some(0),
                },
                publication_revision: 1,
                state: AdmissionAuthorityGuardStateV1::Account {
                    security: FreshAccountSecurityObservationV1 {
                        account_id: "00000000-0000-4000-8000-000000000001".into(),
                        minimum_generation: 1,
                        allowed: true,
                        provenance: test_security(),
                    },
                    presence: None,
                },
            }])
        }
    }
    #[test]
    fn publication_standalone_cannot_acquire_release_or_advance_claims()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let session =
            GameSessionId::decode(&[0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 9])
                .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let character = match game_domain_changes()?.remove(0).key {
            AdmissionAuthorityGuardKeyV1::Character(id) => id,
            _ => unreachable!(),
        };
        for domain in 0..2 {
            for operation in 0..3 {
                let mut prior = if domain == 0 {
                    Owner.resolve_publication(100)?.remove(0)
                } else {
                    game_domain_changes()?.remove(0)
                };
                if operation == 1 {
                    match &mut prior.state {
                        AdmissionAuthorityGuardStateV1::Account { presence, .. } => {
                            *presence = Some((character, session))
                        }
                        AdmissionAuthorityGuardStateV1::Character { holder, .. } => {
                            *holder = Some(session)
                        }
                        _ => unreachable!(),
                    }
                }
                let mut next = prior.clone();
                next.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: 1,
                };
                next.publication_revision = 2;
                next.source.source_revision = 8;
                next.source.decision_identity = "claim-eight".into();
                match &mut next.state {
                    AdmissionAuthorityGuardStateV1::Account { presence, security } => {
                        security.provenance.publication_revision = 2;
                        *presence = if operation == 1 {
                            None
                        } else {
                            Some((character, session))
                        };
                    }
                    AdmissionAuthorityGuardStateV1::Character {
                        holder,
                        lease_generation,
                        ..
                    } => {
                        if operation == 2 {
                            *lease_generation += 1;
                        } else {
                            *holder = if operation == 1 { None } else { Some(session) };
                        }
                    }
                    _ => unreachable!(),
                }
                let request =
                    AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(vec![next]), 100)?;
                assert!(
                    request.validate_locked(&[Some(prior)]).is_err(),
                    "domain {domain}, operation {operation}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn publication_standalone_cannot_substitute_character_claim_owner()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let prior = game_domain_changes()?.remove(0);
        let mut next = prior.clone();
        next.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 1,
        };
        next.publication_revision = 2;
        next.source.source_revision = 8;
        next.source.decision_identity = "new-owner-eight".into();
        if let AdmissionAuthorityGuardStateV1::Character { account_id, .. } = &mut next.state {
            *account_id = "00000000-0000-4000-8000-000000000002".into();
        }
        let request = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(vec![next]), 100)?;
        assert!(request.validate_locked(&[Some(prior)]).is_err());
        Ok(())
    }

    #[test]
    fn publication_bootstrap_cannot_restore_an_occupied_claim()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let bytes = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 9];
        let session = GameSessionId::decode(&bytes)
            .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let character = CharacterId::decode(&bytes)
            .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        for domain in 0..2 {
            let mut row = if domain == 0 {
                Owner.resolve_publication(100)?.remove(0)
            } else {
                game_domain_changes()?.remove(0)
            };
            match &mut row.state {
                AdmissionAuthorityGuardStateV1::Account { presence, .. } => {
                    *presence = Some((character, session))
                }
                AdmissionAuthorityGuardStateV1::Character { holder, .. } => *holder = Some(session),
                _ => unreachable!(),
            }
            let request = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(vec![row]), 100)?;
            assert!(request.validate_locked(&[None]).is_err());
        }
        Ok(())
    }

    #[test]
    fn publication_accepts_independently_owned_bootstrap() {
        assert!(AdmissionAuthorityPublicationV1::prepare(&Owner, 100).is_ok());
    }
    struct ChangedOwner(Vec<AdmissionAuthorityPublicationChangeV1>);
    impl fresh_source_sealed::Sealed for ChangedOwner {}
    impl AdmissionAuthorityOwningPublisherV1 for ChangedOwner {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(self.0.clone())
        }
    }
    struct Queue;
    impl AdmissionAuthorityPublicationPortV1 for Queue {
        fn submit(
            &mut self,
            _request: &AdmissionAuthorityPublicationV1,
        ) -> AdmissionAuthorityPublicationSubmissionV1 {
            AdmissionAuthorityPublicationSubmissionV1::Accepted
        }
        fn reconcile(
            &mut self,
            _request: &AdmissionAuthorityPublicationV1,
        ) -> AdmissionAuthorityPublicationSubmissionV1 {
            AdmissionAuthorityPublicationSubmissionV1::Accepted
        }
    }
    struct Current(Vec<AdmissionAuthorityPublicationChangeV1>);
    impl fresh_source_sealed::Sealed for Current {}
    impl AdmissionAuthorityPublicationCurrentSourceV1 for Current {
        fn current_publications(
            &self,
            _keys: &[AdmissionAuthorityGuardKeyV1],
        ) -> Result<
            Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
            AdmissionAuthorityPublicationErrorV1,
        > {
            Ok(self.0.iter().cloned().map(Some).collect())
        }
    }
    struct DurableCompletion(AdmissionAuthorityPublicationV1);
    impl fresh_source_sealed::Sealed for DurableCompletion {}
    impl AdmissionAuthorityPublicationCompletionSourceV1 for DurableCompletion {
        fn accepted_publication(
            &self,
        ) -> Result<AdmissionAuthorityPublicationV1, AdmissionAuthorityPublicationErrorV1> {
            Ok(self.0.clone())
        }
    }
    #[test]
    fn publication_pending_does_not_activate_readiness()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let prepared = AdmissionAuthorityPublicationV1::prepare(&Owner, 100)?;
        let completion = DurableCompletion(prepared.clone());
        let receipt = AdmissionAuthorityPublicationReceiptV1::resolve(&completion)?;
        let mut flow = AdmissionAuthorityPublicationFlowV1::new(prepared);
        assert_eq!(
            flow.accept(&receipt, &Current(Owner.resolve_publication(100)?)),
            Err(AdmissionAuthorityPublicationErrorV1::WrongPhase)
        );
        flow.submit(&mut Queue)?;
        assert!(flow.active_publication().is_none());
        flow.ambiguous()?;
        flow.reconcile(&mut Queue)?;
        assert!(flow.active_publication().is_none());
        flow.accept(&receipt, &Current(Owner.resolve_publication(100)?))?;
        assert!(flow.active_publication().is_some());
        assert_eq!(
            flow.accept(&receipt, &Current(Owner.resolve_publication(100)?)),
            Err(AdmissionAuthorityPublicationErrorV1::WrongPhase)
        );
        Ok(())
    }
    #[test]
    fn publication_exact_replay_preserves_source_time()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let independently_stored = Owner.resolve_publication(100)?.remove(0);
        let prepared = AdmissionAuthorityPublicationV1::prepare(&Owner, 100)?;
        prepared.validate_locked(&[Some(independently_stored.clone())])?;
        let mut changed = Owner.resolve_publication(100)?;
        changed[0].source.source_observed_at = 101;
        let reread = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(changed), 101)?;
        assert_eq!(
            reread.validate_locked(&[Some(independently_stored)]),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        assert_eq!(prepared.changes()[0].source.source_observed_at, 100);
        Ok(())
    }
    #[test]
    fn publication_stale_cas_and_equal_revision_contradiction_reject()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let prior = Owner.resolve_publication(100)?.remove(0);
        let mut next = Owner.resolve_publication(100)?;
        next[0].precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 1,
        };
        next[0].publication_revision = 2;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut next[0].state {
            security.provenance.publication_revision = 2;
        }
        next[0].source.source_revision = 8;
        next[0].source.decision_identity = "decision-eight".into();
        AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(next.clone()), 100)?
            .validate_locked(&[Some(prior.clone())])?;
        next[0].source.source_revision = 7;
        let contradictory =
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(next.clone()), 100)?;
        assert_eq!(
            contradictory.validate_locked(&[Some(prior.clone())]),
            Err(AdmissionAuthorityPublicationErrorV1::Conflict)
        );
        next[0].source.source_revision = 6;
        let stale = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(next), 100)?;
        assert_eq!(
            stale.validate_locked(&[Some(prior)]),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        Ok(())
    }
    #[test]
    fn publication_missing_bootstrap_stays_closed()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut changes = Owner.resolve_publication(100)?;
        changes[0].precondition = AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: None,
        };
        assert_eq!(
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(changes), 100),
            Err(AdmissionAuthorityPublicationErrorV1::Invalid)
        );
        let prepared = AdmissionAuthorityPublicationV1::prepare(&Owner, 100)?;
        prepared.validate_locked(&[None])?;
        Ok(())
    }
    #[test]
    fn publication_old_receipt_cannot_reactivate_after_newer_deny()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut independent_current = Owner.resolve_publication(100)?;
        independent_current[0].publication_revision = 2;
        independent_current[0].source.source_revision = 8;
        independent_current[0].source.decision_identity = "denied-eight".into();
        independent_current[0].state = AdmissionAuthorityGuardStateV1::Account {
            security: FreshAccountSecurityObservationV1 {
                account_id: "00000000-0000-4000-8000-000000000001".into(),
                minimum_generation: 2,
                allowed: false,
                provenance: test_security(),
            },
            presence: None,
        };
        let prepared = AdmissionAuthorityPublicationV1::prepare(&Owner, 100)?;
        let receipt =
            AdmissionAuthorityPublicationReceiptV1::resolve(&DurableCompletion(prepared.clone()))?;
        let mut flow = AdmissionAuthorityPublicationFlowV1::new(prepared);
        flow.submit(&mut Queue)?;
        assert_eq!(
            flow.accept(&receipt, &Current(independent_current)),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        assert!(flow.active_publication().is_none());
        Ok(())
    }
    #[test]
    fn publication_wrong_domain_and_missing_provenance_reject_independently()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        for mutation in 0..6 {
            let mut independent = Owner.resolve_publication(100)?;
            match mutation {
                0 => independent[0].source.authority.clear(),
                1 => {
                    independent[0].source.purpose =
                        AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                }
                2 => independent[0].source.source_observed_at = 101,
                3 => independent[0].source.clock_uncertainty_seconds = 6,
                4 => independent[0].source.decision_identity.clear(),
                _ => {
                    independent[0].state = AdmissionAuthorityGuardStateV1::SigningTrust {
                        public_key: [1; 32],
                        trusted: true,
                    }
                }
            }
            assert_eq!(
                AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(independent), 100),
                Err(AdmissionAuthorityPublicationErrorV1::Invalid),
                "mutation {mutation}"
            );
        }
        Ok(())
    }
    #[test]
    fn publication_presence_change_cannot_refresh_security_source()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut independent = Owner.resolve_publication(100)?;
        independent[0].source.source_observed_at = 106;
        independent[0].source.source_revision = 8;
        independent[0].source.decision_identity = "game-presence-eight".into();
        assert_eq!(
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(independent), 106),
            Err(AdmissionAuthorityPublicationErrorV1::Invalid)
        );
        Ok(())
    }
    #[test]
    fn publication_advancing_source_cannot_lower_security_high_water()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut prior = Owner.resolve_publication(100)?.remove(0);
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut prior.state {
            security.minimum_generation = 2;
        }
        let mut proposed = Owner.resolve_publication(100)?;
        proposed[0].precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 1,
        };
        proposed[0].publication_revision = 2;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut proposed[0].state {
            security.provenance.publication_revision = 2;
        }
        proposed[0].source.source_revision = 8;
        proposed[0].source.decision_identity = "decision-eight".into();
        let request = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(proposed), 100)?;
        assert_eq!(
            request.validate_locked(&[Some(prior)]),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        Ok(())
    }
    #[test]
    fn publication_atomic_batch_rejects_any_missing_current_guard()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut independent = Owner.resolve_publication(100)?;
        let mut other = independent[0].clone();
        other.key = AdmissionAuthorityGuardKeyV1::Account {
            account_id: "00000000-0000-4000-8000-000000000002".into(),
        };
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut other.state {
            security.account_id = "00000000-0000-4000-8000-000000000002".into();
        }
        independent.push(other);
        let request =
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(independent.clone()), 100)?;
        request.validate_locked(&[None, None])?;
        let receipt =
            AdmissionAuthorityPublicationReceiptV1::resolve(&DurableCompletion(request.clone()))?;
        let mut flow = AdmissionAuthorityPublicationFlowV1::new(request);
        flow.submit(&mut Queue)?;
        independent.pop();
        assert_eq!(
            flow.accept(&receipt, &Current(independent)),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        assert!(flow.active_publication().is_none());
        Ok(())
    }
    fn game_domain_changes()
    -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        let bytes = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 9];
        let world =
            WorldId::decode(&bytes).map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let character = CharacterId::decode(&bytes)
            .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let channel = super::super::ChannelId::decode(&bytes)
            .map_err(|_| AdmissionAuthorityPublicationErrorV1::Invalid)?;
        let source = |purpose| AdmissionPublicationSourceV1 {
            authority: "externally-granted-game-owner".into(),
            purpose,
            source_revision: 7,
            decision_identity: "external-grant-seven".into(),
            source_observed_at: 100,
            clock_uncertainty_seconds: 0,
        };
        let bootstrap = AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        };
        Ok(vec![
            AdmissionAuthorityPublicationChangeV1 {
                key: AdmissionAuthorityGuardKeyV1::Character(character),
                source: source(AdmissionPublicationPurposeV1::CharacterOwnershipAndLease),
                precondition: bootstrap,
                publication_revision: 1,
                state: AdmissionAuthorityGuardStateV1::Character {
                    account_id: "00000000-0000-4000-8000-000000000001".into(),
                    world_id: world,
                    eligible: true,
                    lease_generation: 2,
                    holder: None,
                },
            },
            AdmissionAuthorityPublicationChangeV1 {
                key: AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(
                    world, channel,
                )),
                source: source(AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness),
                precondition: bootstrap,
                publication_revision: 1,
                state: AdmissionAuthorityGuardStateV1::Runtime {
                    ownership_generation: 2,
                    ready: true,
                    route_revision: "route-1".into(),
                    runtime_observation_revision: "runtime-1".into(),
                    protocol_major: 1,
                    transport_profile: 1,
                    ruleset_revision: "rules-1".into(),
                    content_revision: "content-1".into(),
                    map_revision: "map-1".into(),
                    world_policy_revision: "policy-1".into(),
                    offer_revision: "offer-1".into(),
                },
            },
            AdmissionAuthorityPublicationChangeV1 {
                key: AdmissionAuthorityGuardKeyV1::SigningTrust {
                    key_id: "fresh-1".into(),
                    profile: "oteryn-pre-admission-v1".into(),
                },
                source: source(AdmissionPublicationPurposeV1::FixedFreshSigningTrust),
                precondition: bootstrap,
                publication_revision: 1,
                state: AdmissionAuthorityGuardStateV1::SigningTrust {
                    public_key: [7; 32],
                    trusted: true,
                },
            },
        ])
    }
    #[test]
    fn publication_all_domains_preserve_separate_typed_guards()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut independent = Owner.resolve_publication(100)?;
        independent.extend(game_domain_changes()?);
        let publication =
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(independent), 100)?;
        publication.validate_locked(&[None, None, None, None])?;
        assert_eq!(publication.changes().len(), 4);
        Ok(())
    }
    #[test]
    fn publication_advancing_source_cannot_rollback_lease_or_runtime_grant()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        for domain in 0..2 {
            let prior = game_domain_changes()?.remove(domain);
            let mut proposed = game_domain_changes()?.remove(domain);
            proposed.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: 1,
            };
            proposed.publication_revision = 2;
            proposed.source.source_revision = 8;
            proposed.source.decision_identity = "external-eight".into();
            match &mut proposed.state {
                AdmissionAuthorityGuardStateV1::Character {
                    lease_generation, ..
                } => *lease_generation = 1,
                AdmissionAuthorityGuardStateV1::Runtime {
                    ownership_generation,
                    ..
                } => *ownership_generation = 1,
                _ => unreachable!(),
            }
            let request =
                AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(vec![proposed]), 100)?;
            assert_eq!(
                request.validate_locked(&[Some(prior)]),
                Err(AdmissionAuthorityPublicationErrorV1::Stale)
            );
        }
        Ok(())
    }
    #[test]
    fn publication_account_security_subject_substitution_rejects()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut independent = Owner.resolve_publication(100)?;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut independent[0].state
        {
            security.account_id = "00000000-0000-4000-8000-000000000099".into();
        }
        assert_eq!(
            AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(independent), 100),
            Err(AdmissionAuthorityPublicationErrorV1::Invalid)
        );
        Ok(())
    }
    #[test]
    fn publication_tombstone_survives_conflicting_bootstrap()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let mut persisted = Owner.resolve_publication(100)?.remove(0);
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut persisted.state {
            security.allowed = false;
        }
        let bootstrap = AdmissionAuthorityPublicationV1::prepare(&Owner, 100)?;
        assert_eq!(
            bootstrap.validate_locked(&[Some(persisted)]),
            Err(AdmissionAuthorityPublicationErrorV1::Stale)
        );
        Ok(())
    }
    #[test]
    fn publication_game_update_preserves_security_observation_at_new_guard_revision()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let prior = Owner.resolve_publication(100)?.remove(0);
        let mut next = Owner.resolve_publication(100)?;
        next[0].precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 1,
        };
        next[0].publication_revision = 2;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut next[0].state {
            security.provenance.publication_revision = 2;
        }
        next[0].source.source_revision = 8;
        next[0].source.decision_identity = "game-eight".into();
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut next[0].state {
            security.provenance.publication_revision = 2;
        }
        let prepared = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(next), 100)?;
        prepared.validate_locked(&[Some(prior)])?;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } =
            &prepared.changes()[0].state
        {
            assert_eq!(security.provenance.source_observed_at, 100);
            assert_eq!(security.provenance.source_revision, 7);
        }
        Ok(())
    }
    fn reject_security_revision_mutation(
        mutation: usize,
    ) -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        let prior = Owner.resolve_publication(100)?.remove(0);
        let mut next = Owner.resolve_publication(100)?;
        next[0].precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 1,
        };
        next[0].publication_revision = 2;
        next[0].source.source_revision = 8;
        next[0].source.decision_identity = "game-eight".into();
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut next[0].state {
            security.provenance.publication_revision = 2;
            security.provenance.source_revision = 8;
            security.provenance.accepted_source_revision = 8;
            security.provenance.decision_identity = "platform-eight".into();
            security.provenance.accepted_decision_identity = "platform-eight".into();
            if mutation == 0 {
                security.provenance.source_authority = "different-platform-authority".into();
            } else {
                security.provenance.source_observed_at = 99;
            }
        }
        let prepared = AdmissionAuthorityPublicationV1::prepare(&ChangedOwner(next), 100)?;
        assert_eq!(
            prepared.validate_locked(&[Some(prior)]),
            Err(AdmissionAuthorityPublicationErrorV1::Stale),
            "mutation {mutation}"
        );
        Ok(())
    }
    #[test]
    fn publication_security_revision_cannot_cross_authority()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        reject_security_revision_mutation(0)
    }
    #[test]
    fn publication_security_revision_cannot_rollback_source_time()
    -> Result<(), AdmissionAuthorityPublicationErrorV1> {
        reject_security_revision_mutation(1)
    }
}
