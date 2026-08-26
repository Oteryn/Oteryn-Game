use super::{CommandId, MAX_OUTSTANDING_COMMANDS};

/// Current fenced authority required to reconstruct one GameSession projection
/// after process replacement.
///
/// `commit` remains the immutable fresh-admission receipt. Every other field is
/// the current authoritative value and must come from the same fenced authority
/// read. Rehydration never treats GameSessionId as bearer proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameSessionAuthoritySnapshot<T: Copy + Eq> {
    commit: FreshAdmissionCommit<T>,
    session_state: GameSessionState,
    current_connection_generation: ConnectionGeneration,
    current_transport: Option<T>,
    current_character_lease: CharacterLease,
    current_scope_generation: ScopeOwnershipGeneration,
}

impl<T: Copy + Eq> GameSessionAuthoritySnapshot<T> {
    #[must_use]
    pub const fn new(
        commit: FreshAdmissionCommit<T>,
        session_state: GameSessionState,
        current_connection_generation: ConnectionGeneration,
        current_transport: Option<T>,
        current_character_lease: CharacterLease,
        current_scope_generation: ScopeOwnershipGeneration,
    ) -> Self {
        Self {
            commit,
            session_state,
            current_connection_generation,
            current_transport,
            current_character_lease,
            current_scope_generation,
        }
    }

    #[must_use]
    pub const fn commit(self) -> FreshAdmissionCommit<T> {
        self.commit
    }

    #[must_use]
    pub const fn session_state(self) -> GameSessionState {
        self.session_state
    }

    #[must_use]
    pub const fn current_connection_generation(self) -> ConnectionGeneration {
        self.current_connection_generation
    }

    #[must_use]
    pub const fn current_transport(self) -> Option<T> {
        self.current_transport
    }

    #[must_use]
    pub const fn current_character_lease(self) -> CharacterLease {
        self.current_character_lease
    }

    #[must_use]
    pub const fn current_scope_generation(self) -> ScopeOwnershipGeneration {
        self.current_scope_generation
    }
}

impl CharacterLease {
    pub fn new(character_id: CharacterId, generation: u64) -> Result<Self, AdmissionError> {
        if generation == 0 {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self {
            character_id,
            generation,
        })
    }
}

fn validate_current_authority<T: Copy + Eq>(
    expected_game_session_id: GameSessionId,
    snapshot: GameSessionAuthoritySnapshot<T>,
) -> Result<(), AdmissionError> {
    let committed = snapshot.commit();
    if committed.game_session_id() != expected_game_session_id {
        return Err(AdmissionError::ReconciliationUnavailable);
    }

    let lease = snapshot.current_character_lease();
    if lease.character_id() != committed.character_id() {
        return Err(AdmissionError::StaleLease);
    }
    // A same-GameSession recovery may reconstruct current placement/runtime
    // authority, but it cannot adopt a different CharacterLease generation.
    // Any lease generation change is a superseding character-authority fence.
    if lease.generation() != committed.character_lease_generation() {
        return Err(AdmissionError::StaleLease);
    }
    if snapshot.current_scope_generation().get() < committed.scope_ownership_generation() {
        return Err(AdmissionError::StaleRuntime);
    }
    if snapshot.current_connection_generation().get() < committed.connection_generation().get() {
        return Err(AdmissionError::StaleConnection);
    }

    match snapshot.session_state() {
        GameSessionState::Active if snapshot.current_transport().is_some() => Ok(()),
        GameSessionState::Reconnectable if snapshot.current_transport().is_none() => Ok(()),
        GameSessionState::Terminal if snapshot.current_transport().is_none() => Ok(()),
        _ => Err(AdmissionError::ReconciliationUnavailable),
    }
}

impl<T: Copy + Eq, J: ReconnectAttemptJournal<T>> AdmissionAuthority<T, J> {
    pub(super) const fn journal(&self) -> &J {
        &self.reconnect_attempts
    }

    pub(super) fn prepared_reconnect_projection(
        &self,
    ) -> Option<(ReconnectAttemptRef, T, ConnectionGeneration)> {
        self.prepared
            .map(|prepared| (prepared.attempt, prepared.candidate_transport, prepared.candidate))
    }

    pub(super) fn restore_reconciled_prepared_projection(
        &mut self,
        attempt: ReconnectAttemptRef,
        binding: ReconnectCommitBinding<T>,
    ) -> Result<ConnectionGeneration, AdmissionError> {
        if self.control_loss_pending.is_some() || self.runtime_scope_reconciliation_pending {
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        let session = self.current.as_ref().ok_or(AdmissionError::Terminal)?;
        if session.state == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }
        if session.state != GameSessionState::Reconnectable
            || self.current_transport.is_some()
            || session.connection_generation() != binding.predecessor_generation()
            || binding
                .predecessor_generation()
                .get()
                .checked_add(1)
                != Some(binding.candidate_generation().get())
        {
            return Err(AdmissionError::StaleConnection);
        }
        if session.character_lease() != binding.character_lease() {
            return Err(AdmissionError::StaleLease);
        }
        if session.runtime_scope_generation() != binding.scope_generation() {
            return Err(AdmissionError::StaleRuntime);
        }

        let prepared = PreparedReconnect {
            attempt,
            predecessor: binding.predecessor_generation(),
            candidate: binding.candidate_generation(),
            candidate_transport: binding.candidate_transport(),
            lease_generation: binding.character_lease().generation(),
            scope_generation: binding.scope_generation().get(),
        };
        match self.prepared {
            Some(current) if current == prepared => Ok(prepared.candidate),
            Some(_) => Err(AdmissionError::ReconciliationUnavailable),
            None => {
                self.prepared = Some(prepared);
                Ok(prepared.candidate)
            }
        }
    }

    pub(super) fn clear_process_projection(&mut self) {
        if let Some(session) = self.current.as_mut() {
            session.runtime_scope.invalidate();
        }
        self.current = None;
        self.current_transport = None;
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
    }

    pub(super) fn refresh_fresh_projection(
        &mut self,
        authenticated_transport: T,
        snapshot: GameSessionAuthoritySnapshot<T>,
    ) -> Result<&GameSession, AdmissionError> {
        let committed = snapshot.commit();
        validate_current_authority(committed.game_session_id(), snapshot)?;

        if committed.initial_transport() != authenticated_transport
            || snapshot.session_state() != GameSessionState::Active
            || snapshot.current_connection_generation() != committed.connection_generation()
            || snapshot.current_transport() != Some(authenticated_transport)
        {
            self.clear_process_projection();
            return Err(AdmissionError::GrantReplayed);
        }

        let session = self
            .current
            .as_mut()
            .ok_or(AdmissionError::ReconciliationUnavailable)?;
        if session.game_session_id != committed.game_session_id()
            || session.character_id != committed.character_id()
            || session.world_id != committed.world_id()
            || session.channel_id != committed.channel_id()
            || session.connection_generation() != snapshot.current_connection_generation()
        {
            self.clear_process_projection();
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        session.lease_generation = snapshot.current_character_lease().generation();
        let current_scope = session.runtime_scope.generation();
        let authoritative_scope = snapshot.current_scope_generation();
        if authoritative_scope < current_scope {
            self.clear_process_projection();
            return Err(AdmissionError::StaleRuntime);
        }
        if authoritative_scope > current_scope {
            session
                .runtime_scope
                .apply_external_grant(authoritative_scope)
                .map_err(|_| AdmissionError::StaleRuntime)?;
        }
        session.state = GameSessionState::Active;
        self.current_transport = Some(authenticated_transport);
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        self.current.as_ref().ok_or(AdmissionError::ReconciliationUnavailable)
    }

    pub(super) fn install_rehydrated_session(
        &mut self,
        game_session_id: GameSessionId,
        snapshot: GameSessionAuthoritySnapshot<T>,
    ) -> Result<&GameSession, AdmissionError> {
        if let Some(current) = self.current.as_ref() {
            return if current.state == GameSessionState::Terminal {
                Err(AdmissionError::Terminal)
            } else {
                Err(AdmissionError::IncumbentHealthy)
            };
        }
        if self.prepared.is_some()
            || self.control_loss_pending.is_some()
            || self.runtime_scope_reconciliation_pending
        {
            return Err(AdmissionError::ReconciliationUnavailable);
        }

        validate_current_authority(game_session_id, snapshot)?;
        if snapshot.session_state() == GameSessionState::Terminal {
            return Err(AdmissionError::Terminal);
        }

        let committed = snapshot.commit();
        let current_lease = snapshot.current_character_lease();
        let runtime_scope = ScopeRuntimeFence::from_external_grant(snapshot.current_scope_generation());
        let connection = ConnectionFence {
            current: snapshot.current_connection_generation(),
        };

        self.current = Some(GameSession {
            game_session_id,
            character_id: committed.character_id(),
            world_id: committed.world_id(),
            channel_id: committed.channel_id(),
            lease_generation: current_lease.generation(),
            runtime_scope,
            connection,
            state: snapshot.session_state(),
        });
        self.current_transport = snapshot.current_transport();
        self.prepared = None;
        self.control_loss_pending = None;
        self.runtime_scope_reconciliation_pending = false;
        self.current.as_ref().ok_or(AdmissionError::ReconciliationUnavailable)
    }
}

/// Stable non-process-local equality/fencing reference for one authenticated
/// reconnect candidate transport. The bytes carry no chronology or authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthenticatedTransportRefV1([u8; 16]);

impl AuthenticatedTransportRefV1 {
    pub fn decode(input: &[u8]) -> Result<Self, AdmissionError> {
        let bytes: [u8; 16] = input.try_into().map_err(|_| AdmissionError::InvalidFacts)?;
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(AdmissionError::InvalidFacts);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

const RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1: usize = 8;
const MAX_STATE_DOMAINS_PER_RECONNECT_V1: usize = 256;
const EVIDENCE_FRESHNESS_SECONDS_V1: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurabilityErrorV1 {
    InvalidRecord,
    IdempotencyConflict,
    AttemptCapacityExceeded,
    ConcurrentPrepared,
    CompletionMismatch,
    InvalidPhase,
    StaleAuthority,
    DeadlineExpired,
    ReconciliationMismatch,
}

fn non_empty_visible_ascii(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn canonical_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return false;
            }
        } else if !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return false;
        }
    }
    bytes.iter().any(|byte| *byte != b'0' && *byte != b'-')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlLossEpochRefV1(u64);

impl ControlLossEpochRefV1 {
    pub fn new(value: u64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if value == 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeScopeRefV1 {
    Channel {
        world_id: WorldId,
        channel_id: ChannelId,
    },
    Instance {
        world_id: WorldId,
        instance_id: [u8; 16],
    },
}

impl RuntimeScopeRefV1 {
    #[must_use]
    pub const fn channel(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self::Channel {
            world_id,
            channel_id,
        }
    }

    pub fn instance(
        world_id: WorldId,
        instance_id: [u8; 16],
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if instance_id.iter().all(|byte| *byte == 0)
            || instance_id[6] >> 4 != 7
            || instance_id[8] >> 6 != 2
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self::Instance {
            world_id,
            instance_id,
        })
    }

    #[must_use]
    pub const fn world_id(self) -> WorldId {
        match self {
            Self::Channel { world_id, .. } | Self::Instance { world_id, .. } => world_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectIdentityV1 {
    game_session_id: GameSessionId,
    reconnect_attempt_ref: ReconnectAttemptRef,
    account_id: String,
    character_id: CharacterId,
    world_id: WorldId,
    runtime_scope: RuntimeScopeRefV1,
}

impl ReconnectIdentityV1 {
    pub fn new(
        game_session_id: GameSessionId,
        reconnect_attempt_ref: ReconnectAttemptRef,
        account_id: &str,
        character_id: CharacterId,
        world_id: WorldId,
        runtime_scope: RuntimeScopeRefV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !canonical_uuid(account_id) || runtime_scope.world_id() != world_id {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            game_session_id,
            reconnect_attempt_ref,
            account_id: account_id.to_owned(),
            character_id,
            world_id,
            runtime_scope,
        })
    }

    #[must_use]
    pub const fn game_session_id(&self) -> GameSessionId {
        self.game_session_id
    }

    #[must_use]
    pub const fn reconnect_attempt_ref(&self) -> ReconnectAttemptRef {
        self.reconnect_attempt_ref
    }

    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    #[must_use]
    pub const fn character_id(&self) -> CharacterId {
        self.character_id
    }

    #[must_use]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    #[must_use]
    pub const fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.runtime_scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectConnectionFenceV1 {
    predecessor: ConnectionGeneration,
    candidate: ConnectionGeneration,
    transport_ref: AuthenticatedTransportRefV1,
}

impl ReconnectConnectionFenceV1 {
    pub fn new(
        predecessor: ConnectionGeneration,
        candidate: ConnectionGeneration,
        transport_ref: AuthenticatedTransportRefV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if predecessor.get().checked_add(1) != Some(candidate.get()) {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            predecessor,
            candidate,
            transport_ref,
        })
    }

    #[must_use]
    pub const fn predecessor(self) -> ConnectionGeneration {
        self.predecessor
    }

    #[must_use]
    pub const fn candidate(self) -> ConnectionGeneration {
        self.candidate
    }

    #[must_use]
    pub const fn transport_ref(self) -> AuthenticatedTransportRefV1 {
        self.transport_ref
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectAuthorityFenceV1 {
    character_lease_generation: u64,
    scope_ownership_generation: ScopeOwnershipGeneration,
}

impl ReconnectAuthorityFenceV1 {
    pub fn new(
        character_lease_generation: u64,
        scope_ownership_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if character_lease_generation == 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            character_lease_generation,
            scope_ownership_generation,
        })
    }

    #[must_use]
    pub const fn character_lease_generation(self) -> u64 {
        self.character_lease_generation
    }

    #[must_use]
    pub const fn scope_ownership_generation(self) -> ScopeOwnershipGeneration {
        self.scope_ownership_generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionEntitlementV1 {
    Unused,
    Fenced { generation: u64 },
}

impl ProtectionEntitlementV1 {
    #[must_use]
    pub const fn unused() -> Self {
        Self::Unused
    }

    pub fn fenced(generation: u64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if generation == 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self::Fenced { generation })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectContinuityV1 {
    control_loss_epoch: ControlLossEpochRefV1,
    original_grace_deadline: i64,
    prepared_deadline: i64,
    protection_entitlement: ProtectionEntitlementV1,
}

impl ReconnectContinuityV1 {
    pub fn new(
        control_loss_epoch: ControlLossEpochRefV1,
        original_grace_deadline: i64,
        prepared_deadline: i64,
        protection_entitlement: ProtectionEntitlementV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if prepared_deadline <= 0
            || original_grace_deadline <= 0
            || prepared_deadline > original_grace_deadline
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            control_loss_epoch,
            original_grace_deadline,
            prepared_deadline,
            protection_entitlement,
        })
    }

    #[must_use]
    pub const fn control_loss_epoch(self) -> ControlLossEpochRefV1 {
        self.control_loss_epoch
    }

    #[must_use]
    pub const fn original_grace_deadline(self) -> i64 {
        self.original_grace_deadline
    }

    #[must_use]
    pub const fn prepared_deadline(self) -> i64 {
        self.prepared_deadline
    }

    #[must_use]
    pub const fn protection_entitlement(self) -> ProtectionEntitlementV1 {
        self.protection_entitlement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectProofV1 {
    FastReconnect { reconnect_proof_generation: u64 },
    ReauthenticatedRecovery { recovery_grant_nonce: [u8; 32] },
}

impl ReconnectProofV1 {
    fn validate(&self) -> bool {
        match self {
            Self::FastReconnect {
                reconnect_proof_generation,
            } => *reconnect_proof_generation != 0,
            Self::ReauthenticatedRecovery {
                recovery_grant_nonce,
            } => recovery_grant_nonce.iter().any(|byte| *byte != 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingCommandDispositionV1 {
    PendingOriginal,
    TerminalOutcomeRetained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCommandReconciliationV1 {
    command_id: CommandId,
    disposition: PendingCommandDispositionV1,
}

impl PendingCommandReconciliationV1 {
    #[must_use]
    pub const fn new(command_id: CommandId, disposition: PendingCommandDispositionV1) -> Self {
        Self {
            command_id,
            disposition,
        }
    }

    #[must_use]
    pub const fn command_id(self) -> CommandId {
        self.command_id
    }

    #[must_use]
    pub const fn disposition(self) -> PendingCommandDispositionV1 {
        self.disposition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateDomainRevisionV1 {
    domain_id: u32,
    revision: u64,
}

impl StateDomainRevisionV1 {
    pub fn new(domain_id: u32, revision: u64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if domain_id == 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            domain_id,
            revision,
        })
    }

    #[must_use]
    pub const fn domain_id(self) -> u32 {
        self.domain_id
    }

    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fnd02ReconciliationFenceV1 {
    next_command_id: CommandId,
    pending: Vec<PendingCommandReconciliationV1>,
    server_sequence: u64,
    domain_revisions: Vec<StateDomainRevisionV1>,
}

impl Fnd02ReconciliationFenceV1 {
    pub fn new(
        next_command_id: CommandId,
        pending: Vec<PendingCommandReconciliationV1>,
        server_sequence: u64,
        domain_revisions: Vec<StateDomainRevisionV1>,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if pending.len() > MAX_OUTSTANDING_COMMANDS
            || domain_revisions.len() > MAX_STATE_DOMAINS_PER_RECONNECT_V1
            || pending
                .iter()
                .any(|item| item.command_id().get() >= next_command_id.get())
            || pending
                .windows(2)
                .any(|pair| pair[0].command_id() >= pair[1].command_id())
            || domain_revisions
                .windows(2)
                .any(|pair| pair[0].domain_id() >= pair[1].domain_id())
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            next_command_id,
            pending,
            server_sequence,
            domain_revisions,
        })
    }

    #[must_use]
    pub const fn next_command_id(&self) -> CommandId {
        self.next_command_id
    }

    #[must_use]
    pub fn pending(&self) -> &[PendingCommandReconciliationV1] {
        &self.pending
    }

    #[must_use]
    pub const fn server_sequence(&self) -> u64 {
        self.server_sequence
    }

    #[must_use]
    pub fn domain_revisions(&self) -> &[StateDomainRevisionV1] {
        &self.domain_revisions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityEvidenceFenceV1 {
    authority: String,
    purpose: String,
    scope: String,
    source_revision: String,
    decision_identity: String,
    source_observed_at: i64,
}

impl AuthorityEvidenceFenceV1 {
    pub fn new(
        authority: &str,
        purpose: &str,
        scope: &str,
        source_revision: &str,
        decision_identity: &str,
        source_observed_at: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if source_observed_at < 0
            || ![
                authority,
                purpose,
                scope,
                source_revision,
                decision_identity,
            ]
            .iter()
            .all(|value| non_empty_visible_ascii(value))
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            authority: authority.to_owned(),
            purpose: purpose.to_owned(),
            scope: scope.to_owned(),
            source_revision: source_revision.to_owned(),
            decision_identity: decision_identity.to_owned(),
            source_observed_at,
        })
    }

    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }

    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    #[must_use]
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    #[must_use]
    pub fn decision_identity(&self) -> &str {
        &self.decision_identity
    }

    #[must_use]
    pub const fn source_observed_at(&self) -> i64 {
        self.source_observed_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCompatibilityEvidenceV1 {
    protocol_major: u32,
    transport_profile: u32,
    ruleset_revision: String,
    content_revision: String,
    map_revision: String,
    world_policy_revision: String,
    account_security_generation: u64,
    platform_security_evidence: AuthorityEvidenceFenceV1,
    proof_trust_evidence: AuthorityEvidenceFenceV1,
    credential_expiration: Option<i64>,
}

impl ReconnectCompatibilityEvidenceV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        protocol_major: u32,
        transport_profile: u32,
        ruleset_revision: &str,
        content_revision: &str,
        map_revision: &str,
        world_policy_revision: &str,
        account_security_generation: u64,
        platform_security_evidence: AuthorityEvidenceFenceV1,
        proof_trust_evidence: AuthorityEvidenceFenceV1,
        credential_expiration: Option<i64>,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if protocol_major == 0
            || transport_profile == 0
            || account_security_generation == 0
            || ![
                ruleset_revision,
                content_revision,
                map_revision,
                world_policy_revision,
            ]
            .iter()
            .all(|value| non_empty_visible_ascii(value))
            || credential_expiration.is_some_and(|value| value <= 0)
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            protocol_major,
            transport_profile,
            ruleset_revision: ruleset_revision.to_owned(),
            content_revision: content_revision.to_owned(),
            map_revision: map_revision.to_owned(),
            world_policy_revision: world_policy_revision.to_owned(),
            account_security_generation,
            platform_security_evidence,
            proof_trust_evidence,
            credential_expiration,
        })
    }

    #[must_use]
    pub const fn protocol_major(&self) -> u32 {
        self.protocol_major
    }

    #[must_use]
    pub const fn transport_profile(&self) -> u32 {
        self.transport_profile
    }

    #[must_use]
    pub fn ruleset_revision(&self) -> &str {
        &self.ruleset_revision
    }

    #[must_use]
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }

    #[must_use]
    pub fn map_revision(&self) -> &str {
        &self.map_revision
    }

    #[must_use]
    pub fn world_policy_revision(&self) -> &str {
        &self.world_policy_revision
    }

    #[must_use]
    pub const fn account_security_generation(&self) -> u64 {
        self.account_security_generation
    }

    #[must_use]
    pub const fn credential_expiration(&self) -> Option<i64> {
        self.credential_expiration
    }

    #[must_use]
    pub const fn platform_security_evidence(&self) -> &AuthorityEvidenceFenceV1 {
        &self.platform_security_evidence
    }

    #[must_use]
    pub const fn proof_trust_evidence(&self) -> &AuthorityEvidenceFenceV1 {
        &self.proof_trust_evidence
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurabilityRecordV1 {
    identity: ReconnectIdentityV1,
    connection: ReconnectConnectionFenceV1,
    authority: ReconnectAuthorityFenceV1,
    continuity: ReconnectContinuityV1,
    proof: ReconnectProofV1,
    fnd02: Fnd02ReconciliationFenceV1,
    compatibility: ReconnectCompatibilityEvidenceV1,
}

impl ReconnectDurabilityRecordV1 {
    pub fn new(
        identity: ReconnectIdentityV1,
        connection: ReconnectConnectionFenceV1,
        authority: ReconnectAuthorityFenceV1,
        continuity: ReconnectContinuityV1,
        proof: ReconnectProofV1,
        fnd02: Fnd02ReconciliationFenceV1,
        compatibility: ReconnectCompatibilityEvidenceV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !proof.validate() {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            identity,
            connection,
            authority,
            continuity,
            proof,
            fnd02,
            compatibility,
        })
    }

    #[must_use]
    pub const fn version(&self) -> u16 {
        1
    }

    #[must_use]
    pub const fn identity(&self) -> &ReconnectIdentityV1 {
        &self.identity
    }

    #[must_use]
    pub const fn connection(&self) -> ReconnectConnectionFenceV1 {
        self.connection
    }

    #[must_use]
    pub const fn authority(&self) -> ReconnectAuthorityFenceV1 {
        self.authority
    }

    #[must_use]
    pub const fn continuity(&self) -> ReconnectContinuityV1 {
        self.continuity
    }

    #[must_use]
    pub const fn proof(&self) -> &ReconnectProofV1 {
        &self.proof
    }

    #[must_use]
    pub const fn fnd02(&self) -> &Fnd02ReconciliationFenceV1 {
        &self.fnd02
    }

    #[must_use]
    pub const fn compatibility(&self) -> &ReconnectCompatibilityEvidenceV1 {
        &self.compatibility
    }

    fn authorization_deadline(&self) -> Result<i64, ReconnectDurabilityErrorV1> {
        let compatibility = self.compatibility();
        let platform_deadline = compatibility
            .platform_security_evidence()
            .source_observed_at()
            .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
            .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        let trust_deadline = compatibility
            .proof_trust_evidence()
            .source_observed_at()
            .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
            .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        let mut deadline = self
            .continuity()
            .prepared_deadline()
            .min(self.continuity().original_grace_deadline())
            .min(platform_deadline)
            .min(trust_deadline);
        if let Some(credential_expiration) = compatibility.credential_expiration() {
            deadline = deadline.min(credential_expiration);
        }
        Ok(deadline)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAttemptReservationV1 {
    New,
    Existing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectPrepareDispositionV1 {
    Prepared,
    ExistingPrepared,
    RejectedTransportRefCollision,
    RejectedConcurrentPrepared,
    RejectedStaleAuthority,
    AttemptCapacityExceeded,
    ExistingTerminal,
    Unavailable,
    Ambiguous,
    IdempotencyConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconnectAttemptStateV1 {
    Reserved,
    Prepared,
    CollisionTerminal,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReconnectAttemptEntryV1 {
    attempt: ReconnectAttemptRef,
    transport_ref: AuthenticatedTransportRefV1,
    state: ReconnectAttemptStateV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectAttemptBudgetV1 {
    control_loss_epoch: ControlLossEpochRefV1,
    entries: Vec<ReconnectAttemptEntryV1>,
}

impl ReconnectAttemptBudgetV1 {
    #[must_use]
    pub const fn new(control_loss_epoch: ControlLossEpochRefV1) -> Self {
        Self {
            control_loss_epoch,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub const fn control_loss_epoch(&self) -> ControlLossEpochRefV1 {
        self.control_loss_epoch
    }

    #[must_use]
    pub fn distinct_attempts(&self) -> usize {
        self.entries.len()
    }

    pub fn reserve(
        &mut self,
        attempt: ReconnectAttemptRef,
        transport_ref: AuthenticatedTransportRefV1,
    ) -> Result<ReconnectAttemptReservationV1, ReconnectDurabilityErrorV1> {
        if let Some(entry) = self.entries.iter().find(|entry| entry.attempt == attempt) {
            return if entry.transport_ref == transport_ref {
                Ok(ReconnectAttemptReservationV1::Existing)
            } else {
                Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
            };
        }
        if self.entries.len() >= RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1 {
            return Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded);
        }
        self.entries.push(ReconnectAttemptEntryV1 {
            attempt,
            transport_ref,
            state: ReconnectAttemptStateV1::Reserved,
        });
        Ok(ReconnectAttemptReservationV1::New)
    }

    pub fn accept_prepare_completion(
        &mut self,
        attempt: ReconnectAttemptRef,
        transport_ref: AuthenticatedTransportRefV1,
        disposition: ReconnectPrepareDispositionV1,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.attempt == attempt)
            .ok_or(ReconnectDurabilityErrorV1::CompletionMismatch)?;
        if self.entries[index].transport_ref != transport_ref {
            return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
        }
        match disposition {
            ReconnectPrepareDispositionV1::Prepared
            | ReconnectPrepareDispositionV1::ExistingPrepared => {
                if self.entries.iter().enumerate().any(|(other, entry)| {
                    other != index && entry.state == ReconnectAttemptStateV1::Prepared
                }) {
                    return Err(ReconnectDurabilityErrorV1::ConcurrentPrepared);
                }
                self.entries[index].state = ReconnectAttemptStateV1::Prepared;
            }
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision => {
                self.entries[index].state = ReconnectAttemptStateV1::CollisionTerminal;
            }
            ReconnectPrepareDispositionV1::Unavailable
            | ReconnectPrepareDispositionV1::Ambiguous => {}
            ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
            | ReconnectPrepareDispositionV1::RejectedStaleAuthority
            | ReconnectPrepareDispositionV1::AttemptCapacityExceeded
            | ReconnectPrepareDispositionV1::ExistingTerminal
            | ReconnectPrepareDispositionV1::IdempotencyConflict => {
                self.entries[index].state = ReconnectAttemptStateV1::Terminal;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn replacement_allowed_after_collision(&self, attempt: ReconnectAttemptRef) -> bool {
        self.entries.len() < RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1
            && !self
                .entries
                .iter()
                .any(|entry| entry.state == ReconnectAttemptStateV1::Prepared)
            && self.entries.iter().any(|entry| {
                entry.attempt == attempt
                    && entry.state == ReconnectAttemptStateV1::CollisionTerminal
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareRequestV1 {
    record: ReconnectDurabilityRecordV1,
}

impl ReconnectPrepareRequestV1 {
    #[must_use]
    pub const fn record(&self) -> &ReconnectDurabilityRecordV1 {
        &self.record
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareCompletionV1 {
    request: ReconnectPrepareRequestV1,
    disposition: ReconnectPrepareDispositionV1,
}

impl ReconnectPrepareCompletionV1 {
    #[must_use]
    pub fn for_request(
        request: &ReconnectPrepareRequestV1,
        disposition: ReconnectPrepareDispositionV1,
    ) -> Self {
        Self {
            request: request.clone(),
            disposition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectPrepareActionV1 {
    RetrySameRequest(ReconnectPrepareRequestV1),
    AwaitFinalRevalidation,
    ReconcileSameAttempt,
    Terminal(ReconnectPrepareDispositionV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurabilityPhaseV1 {
    PendingPrepare,
    AwaitFinalRevalidation,
    PendingCommit,
    ReconciliationRequired,
    Terminal,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCurrentAuthorityV1 {
    identity: ReconnectIdentityV1,
    predecessor: ConnectionGeneration,
    authority: ReconnectAuthorityFenceV1,
    continuity_epoch: ControlLossEpochRefV1,
    proof: ReconnectProofV1,
    fnd02: Fnd02ReconciliationFenceV1,
    protocol_major: u32,
    transport_profile: u32,
    ruleset_revision: String,
    content_revision: String,
    map_revision: String,
    world_policy_revision: String,
    account_security_generation: u64,
    platform_security_evidence: AuthorityEvidenceFenceV1,
    proof_trust_evidence: AuthorityEvidenceFenceV1,
    credential_expiration: Option<i64>,
    session_state: GameSessionState,
    current_controller_present: bool,
    observed_at: i64,
}

impl ReconnectCurrentAuthorityV1 {
    pub fn from_record(
        record: &ReconnectDurabilityRecordV1,
        observed_at: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if observed_at < 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        let compatibility = record.compatibility();
        Ok(Self {
            identity: record.identity().clone(),
            predecessor: record.connection().predecessor(),
            authority: record.authority(),
            continuity_epoch: record.continuity().control_loss_epoch(),
            proof: record.proof().clone(),
            fnd02: record.fnd02().clone(),
            protocol_major: compatibility.protocol_major(),
            transport_profile: compatibility.transport_profile(),
            ruleset_revision: compatibility.ruleset_revision().to_owned(),
            content_revision: compatibility.content_revision().to_owned(),
            map_revision: compatibility.map_revision().to_owned(),
            world_policy_revision: compatibility.world_policy_revision().to_owned(),
            account_security_generation: compatibility.account_security_generation(),
            platform_security_evidence: compatibility.platform_security_evidence().clone(),
            proof_trust_evidence: compatibility.proof_trust_evidence().clone(),
            credential_expiration: compatibility.credential_expiration(),
            session_state: GameSessionState::Reconnectable,
            current_controller_present: false,
            observed_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectCommitAuthorizationV1 {
    authorization_deadline: i64,
}

impl ReconnectCommitAuthorizationV1 {
    #[must_use]
    pub const fn authorization_deadline(self) -> i64 {
        self.authorization_deadline
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCommitRequestV1 {
    record: ReconnectDurabilityRecordV1,
    authorization: ReconnectCommitAuthorizationV1,
}

impl ReconnectCommitRequestV1 {
    #[must_use]
    pub const fn record(&self) -> &ReconnectDurabilityRecordV1 {
        &self.record
    }

    #[must_use]
    pub const fn authorization(&self) -> ReconnectCommitAuthorizationV1 {
        self.authorization
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectCommitDispositionV1 {
    Committed,
    Unavailable,
    Ambiguous,
    RejectedStaleAuthority,
    ExistingTerminal,
    IdempotencyConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCommitCompletionV1 {
    request: ReconnectCommitRequestV1,
    disposition: ReconnectCommitDispositionV1,
}

impl ReconnectCommitCompletionV1 {
    #[must_use]
    pub fn for_request(
        request: &ReconnectCommitRequestV1,
        disposition: ReconnectCommitDispositionV1,
    ) -> Self {
        Self {
            request: request.clone(),
            disposition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectCommitActionV1 {
    RetrySameRequest(ReconnectCommitRequestV1),
    ReconcileSameAttempt,
    Terminal(ReconnectCommitDispositionV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableReconnectStateV1 {
    Prepared,
    Committed,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurableReconciliationSnapshotV1 {
    record: ReconnectDurabilityRecordV1,
    durable_state: DurableReconnectStateV1,
    current_generation: Option<ConnectionGeneration>,
    current_transport_ref: Option<AuthenticatedTransportRefV1>,
}

impl ReconnectDurableReconciliationSnapshotV1 {
    #[must_use]
    pub fn committed(record: ReconnectDurabilityRecordV1) -> Self {
        Self {
            current_generation: Some(record.connection().candidate()),
            current_transport_ref: Some(record.connection().transport_ref()),
            record,
            durable_state: DurableReconnectStateV1::Committed,
        }
    }

    #[must_use]
    pub fn prepared(record: ReconnectDurabilityRecordV1) -> Self {
        Self {
            current_generation: None,
            current_transport_ref: None,
            record,
            durable_state: DurableReconnectStateV1::Prepared,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectProjectionDecisionV1 {
    InstallController {
        generation: ConnectionGeneration,
        transport_ref: AuthenticatedTransportRefV1,
    },
    AwaitFinalRevalidation,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurabilityFlowV1 {
    record: ReconnectDurabilityRecordV1,
    prepare_request: ReconnectPrepareRequestV1,
    commit_request: Option<ReconnectCommitRequestV1>,
    phase: ReconnectDurabilityPhaseV1,
}

impl ReconnectDurabilityFlowV1 {
    #[must_use]
    pub fn begin(record: ReconnectDurabilityRecordV1) -> (Self, ReconnectPrepareRequestV1) {
        let prepare_request = ReconnectPrepareRequestV1 {
            record: record.clone(),
        };
        (
            Self {
                record,
                prepare_request: prepare_request.clone(),
                commit_request: None,
                phase: ReconnectDurabilityPhaseV1::PendingPrepare,
            },
            prepare_request,
        )
    }

    #[must_use]
    pub const fn phase(&self) -> ReconnectDurabilityPhaseV1 {
        self.phase
    }

    pub fn accept_prepare_completion(
        &mut self,
        completion: ReconnectPrepareCompletionV1,
    ) -> Result<ReconnectPrepareActionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::PendingPrepare {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if completion.request != self.prepare_request {
            return Err(ReconnectDurabilityErrorV1::CompletionMismatch);
        }
        match completion.disposition {
            ReconnectPrepareDispositionV1::Prepared
            | ReconnectPrepareDispositionV1::ExistingPrepared => {
                self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation;
                Ok(ReconnectPrepareActionV1::AwaitFinalRevalidation)
            }
            ReconnectPrepareDispositionV1::Unavailable => Ok(
                ReconnectPrepareActionV1::RetrySameRequest(self.prepare_request.clone()),
            ),
            ReconnectPrepareDispositionV1::Ambiguous => {
                self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired;
                Ok(ReconnectPrepareActionV1::ReconcileSameAttempt)
            }
            terminal => {
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV1::Terminal(terminal))
            }
        }
    }

    pub fn authorize_commit(
        &mut self,
        current: ReconnectCurrentAuthorityV1,
        now: i64,
    ) -> Result<ReconnectCommitRequestV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::AwaitFinalRevalidation {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let expected = ReconnectCurrentAuthorityV1::from_record(&self.record, current.observed_at)?;
        if current != expected
            || current.session_state != GameSessionState::Reconnectable
            || current.current_controller_present
        {
            self.phase = ReconnectDurabilityPhaseV1::Terminal;
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        let deadline = self.record.authorization_deadline()?;
        if now > deadline {
            self.phase = ReconnectDurabilityPhaseV1::Terminal;
            return Err(ReconnectDurabilityErrorV1::DeadlineExpired);
        }
        let request = ReconnectCommitRequestV1 {
            record: self.record.clone(),
            authorization: ReconnectCommitAuthorizationV1 {
                authorization_deadline: deadline,
            },
        };
        self.commit_request = Some(request.clone());
        self.phase = ReconnectDurabilityPhaseV1::PendingCommit;
        Ok(request)
    }

    pub fn accept_commit_completion(
        &mut self,
        completion: ReconnectCommitCompletionV1,
    ) -> Result<ReconnectCommitActionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::PendingCommit {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if self.commit_request.as_ref() != Some(&completion.request) {
            return Err(ReconnectDurabilityErrorV1::CompletionMismatch);
        }
        match completion.disposition {
            ReconnectCommitDispositionV1::Unavailable => Ok(
                ReconnectCommitActionV1::RetrySameRequest(completion.request),
            ),
            ReconnectCommitDispositionV1::Committed
            | ReconnectCommitDispositionV1::Ambiguous => {
                self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired;
                Ok(ReconnectCommitActionV1::ReconcileSameAttempt)
            }
            terminal => {
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectCommitActionV1::Terminal(terminal))
            }
        }
    }

    pub fn accept_reconciliation(
        &mut self,
        snapshot: ReconnectDurableReconciliationSnapshotV1,
        current_scope_generation: ScopeOwnershipGeneration,
    ) -> Result<ReconnectProjectionDecisionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if snapshot.record != self.record
            || current_scope_generation != self.record.authority().scope_ownership_generation()
        {
            return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch);
        }
        match snapshot.durable_state {
            DurableReconnectStateV1::Prepared => {
                if snapshot.current_generation.is_some() || snapshot.current_transport_ref.is_some() {
                    return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch);
                }
                self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation;
                Ok(ReconnectProjectionDecisionV1::AwaitFinalRevalidation)
            }
            DurableReconnectStateV1::Committed => {
                if snapshot.current_generation != Some(self.record.connection().candidate())
                    || snapshot.current_transport_ref
                        != Some(self.record.connection().transport_ref())
                {
                    return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch);
                }
                self.phase = ReconnectDurabilityPhaseV1::Completed;
                Ok(ReconnectProjectionDecisionV1::InstallController {
                    generation: self.record.connection().candidate(),
                    transport_ref: self.record.connection().transport_ref(),
                })
            }
            DurableReconnectStateV1::Terminal => {
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectProjectionDecisionV1::Terminal)
            }
        }
    }
}

#[cfg(test)]
mod durability_reconnect_v1_tests {
    use super::*;

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[8..].copy_from_slice(&raw.to_be_bytes());
        out[6] = 0x70;
        out[8] = (out[8] & 0x3f) | 0x80;
        out
    }

    fn sample_record(
        attempt_raw: u64,
        transport_byte: u8,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let game_session_id = GameSessionId::decode(&uuid_v7(10))
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let character_id = CharacterId::decode(&uuid_v7(11))
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let world_id = WorldId::decode(&uuid_v7(12))
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let channel_id = ChannelId::decode(&uuid_v7(13))
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let identity = ReconnectIdentityV1::new(
            game_session_id,
            ReconnectAttemptRef::new(attempt_raw)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            "123e4567-e89b-12d3-a456-426614174000",
            character_id,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel_id),
        )?;
        let predecessor = ConnectionGeneration::new(7)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let candidate = ConnectionGeneration::new(8)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let connection = ReconnectConnectionFenceV1::new(predecessor, candidate, transport_ref)?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            120,
            115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![
                PendingCommandReconciliationV1::new(
                    CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::TerminalOutcomeRetained,
                ),
            ],
            41,
            vec![
                StateDomainRevisionV1::new(1, 4)?,
                StateDomainRevisionV1::new(2, 7)?,
            ],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            100,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            101,
        )?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            platform,
            trust,
            Some(110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x55; 32],
            },
            fnd02,
            compatibility,
        )
    }

    #[test]
    fn authenticated_transport_ref_v1_is_exact_nonzero_16_bytes() -> Result<(), AdmissionError> {
        let encoded = [0xA5u8; 16];
        let transport_ref = AuthenticatedTransportRefV1::decode(&encoded)?;
        assert_eq!(transport_ref.to_bytes(), encoded);
        assert_eq!(
            AuthenticatedTransportRefV1::decode(&[0u8; 16]),
            Err(AdmissionError::InvalidFacts)
        );
        assert_eq!(
            AuthenticatedTransportRefV1::decode(&[0xA5u8; 15]),
            Err(AdmissionError::InvalidFacts)
        );
        Ok(())
    }

    #[test]
    fn same_attempt_ref_is_immutable_and_ninth_attempt_fails_before_allocation(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let epoch = ControlLossEpochRefV1::new(9)?;
        let mut budget = ReconnectAttemptBudgetV1::new(epoch);
        let first_attempt = ReconnectAttemptRef::new(1)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let first_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let changed_ref = AuthenticatedTransportRefV1::decode(&[2u8; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            budget.reserve(first_attempt, first_ref)?,
            ReconnectAttemptReservationV1::New
        );
        assert_eq!(
            budget.reserve(first_attempt, first_ref)?,
            ReconnectAttemptReservationV1::Existing
        );
        assert_eq!(
            budget.reserve(first_attempt, changed_ref),
            Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
        );
        for raw in 2u64..=8 {
            let attempt = ReconnectAttemptRef::new(raw)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            let transport_ref = AuthenticatedTransportRefV1::decode(&[byte; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            assert_eq!(
                budget.reserve(attempt, transport_ref)?,
                ReconnectAttemptReservationV1::New
            );
        }
        let ninth = ReconnectAttemptRef::new(9)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let ninth_ref = AuthenticatedTransportRefV1::decode(&[9u8; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            budget.reserve(ninth, ninth_ref),
            Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded)
        );
        assert_eq!(budget.distinct_attempts(), 8);
        Ok(())
    }

    #[test]
    fn collision_allows_only_new_attempt_under_capacity_and_never_same_attempt_remint(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(1)?);
        let attempt = ReconnectAttemptRef::new(1)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        budget.reserve(attempt, transport_ref)?;
        budget.accept_prepare_completion(
            attempt,
            transport_ref,
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision,
        )?;
        assert!(budget.replacement_allowed_after_collision(attempt));
        assert_eq!(
            budget.reserve(
                attempt,
                AuthenticatedTransportRefV1::decode(&[2u8; 16])
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ),
            Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
        );
        let replacement = ReconnectAttemptRef::new(2)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            budget.reserve(
                replacement,
                AuthenticatedTransportRefV1::decode(&[2u8; 16])
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            )?,
            ReconnectAttemptReservationV1::New
        );

        let mut exhausted = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(2)?);
        for raw in 1u64..=8 {
            let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            exhausted.reserve(
                ReconnectAttemptRef::new(raw)
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                AuthenticatedTransportRefV1::decode(&[byte; 16])
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            )?;
        }
        let final_attempt = ReconnectAttemptRef::new(8)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let final_ref = AuthenticatedTransportRefV1::decode(&[8u8; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        exhausted.accept_prepare_completion(
            final_attempt,
            final_ref,
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision,
        )?;
        assert!(!exhausted.replacement_allowed_after_collision(final_attempt));
        Ok(())
    }

    #[test]
    fn record_preserves_complete_authority_reconciliation_and_security_evidence(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(3, 3)?;
        assert_eq!(record.version(), 1);
        assert_eq!(
            record.identity().account_id(),
            "123e4567-e89b-12d3-a456-426614174000"
        );
        assert!(matches!(
            record.identity().runtime_scope(),
            RuntimeScopeRefV1::Channel { .. }
        ));
        assert_eq!(record.connection().predecessor().get(), 7);
        assert_eq!(record.connection().candidate().get(), 8);
        assert_eq!(record.authority().character_lease_generation(), 9);
        assert_eq!(record.continuity().control_loss_epoch().get(), 3);
        assert_eq!(record.fnd02().pending().len(), 2);
        assert_eq!(record.fnd02().domain_revisions().len(), 2);
        assert_eq!(record.compatibility().protocol_major(), 1);
        assert_eq!(record.compatibility().transport_profile(), 1);
        assert_eq!(record.compatibility().account_security_generation(), 12);
        assert_eq!(record.compatibility().credential_expiration(), Some(110));
        assert_eq!(
            record
                .compatibility()
                .platform_security_evidence()
                .source_observed_at(),
            100
        );
        assert_eq!(
            record
                .compatibility()
                .proof_trust_evidence()
                .source_observed_at(),
            101
        );
        Ok(())
    }

    #[test]
    fn prepare_unavailable_retries_same_request_and_ambiguous_requires_reconciliation(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(4, 4)?;
        let (mut unavailable_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        let unavailable = ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Unavailable,
        );
        assert_eq!(
            unavailable_flow.accept_prepare_completion(unavailable)?,
            ReconnectPrepareActionV1::RetrySameRequest(request.clone())
        );
        assert_eq!(
            unavailable_flow.phase(),
            ReconnectDurabilityPhaseV1::PendingPrepare
        );

        let (mut ambiguous_flow, ambiguous_request) = ReconnectDurabilityFlowV1::begin(record);
        let ambiguous = ReconnectPrepareCompletionV1::for_request(
            &ambiguous_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        );
        assert_eq!(
            ambiguous_flow.accept_prepare_completion(ambiguous)?,
            ReconnectPrepareActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            ambiguous_flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
        Ok(())
    }

    #[test]
    fn prepared_completion_requires_fresh_complete_revalidation_before_commit(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(5, 5)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        let prepared = ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        );
        assert_eq!(
            flow.accept_prepare_completion(prepared)?,
            ReconnectPrepareActionV1::AwaitFinalRevalidation
        );
        let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        let commit = flow.authorize_commit(current, 104)?;
        assert_eq!(commit.authorization().authorization_deadline(), 105);
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);

        let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        stale_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &stale_request,
            ReconnectPrepareDispositionV1::Prepared,
        ))?;
        let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        stale.account_security_generation = stale
            .account_security_generation
            .checked_add(1)
            .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(
            stale_flow.authorize_commit(stale, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn ambiguous_commit_installs_controller_only_after_exact_committed_reconciliation(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(6, 6)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        ))?;
        let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        let commit_request = flow.authorize_commit(current, 104)?;
        let ambiguous = ReconnectCommitCompletionV1::for_request(
            &commit_request,
            ReconnectCommitDispositionV1::Ambiguous,
        );
        assert_eq!(
            flow.accept_commit_completion(ambiguous)?,
            ReconnectCommitActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );

        let snapshot = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
        assert_eq!(
            flow.accept_reconciliation(
                snapshot,
                record.authority().scope_ownership_generation(),
            )?,
            ReconnectProjectionDecisionV1::InstallController {
                generation: record.connection().candidate(),
                transport_ref: record.connection().transport_ref(),
            }
        );

        let (mut mismatch_flow, mismatch_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        mismatch_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &mismatch_request,
            ReconnectPrepareDispositionV1::Ambiguous,
        ))?;
        let mut mismatch = ReconnectDurableReconciliationSnapshotV1::committed(record);
        mismatch.current_transport_ref = Some(
            AuthenticatedTransportRefV1::decode(&[7u8; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        );
        assert_eq!(
            mismatch_flow.accept_reconciliation(
                mismatch,
                ScopeOwnershipGeneration::new(10)
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        Ok(())
    }
}