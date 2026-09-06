use super::{CommandId, MAX_OUTSTANDING_COMMANDS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterWorldEligibilityClaimV1 {
    character_id: CharacterId,
    world_id: WorldId,
}

impl CharacterWorldEligibilityClaimV1 {
    #[must_use]
    pub const fn new(character_id: CharacterId, world_id: WorldId) -> Self {
        Self {
            character_id,
            world_id,
        }
    }

    #[must_use]
    fn expected_from_identity(identity: &ReconnectIdentityV1) -> Self {
        Self::new(identity.character_id(), identity.world_id())
    }

    #[cfg(test)]
    #[must_use]
    pub fn from_identity(identity: &ReconnectIdentityV1) -> Self {
        Self::expected_from_identity(identity)
    }

    #[must_use]
    pub const fn character_id(self) -> CharacterId {
        self.character_id
    }

    #[must_use]
    pub const fn world_id(self) -> WorldId {
        self.world_id
    }
}

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
    current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
    current_runtime_scope: RuntimeScopeRefV1,
    current_scope_generation: ScopeOwnershipGeneration,
    current_control_loss_epoch: Option<ControlLossEpochRefV1>,
    current_original_grace_deadline: Option<i64>,
}

impl<T: Copy + Eq> GameSessionAuthoritySnapshot<T> {
    #[cfg(test)]
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
            current_character_world_eligibility: Some(CharacterWorldEligibilityClaimV1::new(
                commit.character_id(),
                commit.world_id(),
            )),
            current_runtime_scope: RuntimeScopeRefV1::channel(commit.world_id(), commit.channel_id()),
            current_scope_generation,
            current_control_loss_epoch: None,
            current_original_grace_deadline: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_current_facts(
        commit: FreshAdmissionCommit<T>,
        session_state: GameSessionState,
        current_connection_generation: ConnectionGeneration,
        current_transport: Option<T>,
        current_character_lease: CharacterLease,
        current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
        current_runtime_scope: RuntimeScopeRefV1,
        current_scope_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if current_runtime_scope.world_id() != commit.world_id() {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            commit,
            session_state,
            current_connection_generation,
            current_transport,
            current_character_lease,
            current_character_world_eligibility,
            current_runtime_scope,
            current_scope_generation,
            current_control_loss_epoch: None,
            current_original_grace_deadline: None,
        })
    }

    pub fn with_control_loss_continuity(
        mut self,
        control_loss_epoch: ControlLossEpochRefV1,
        original_grace_deadline: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if original_grace_deadline <= 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        self.current_control_loss_epoch = Some(control_loss_epoch);
        self.current_original_grace_deadline = Some(original_grace_deadline);
        Ok(self)
    }

    pub fn with_current_runtime_scope(
        mut self,
        current_runtime_scope: RuntimeScopeRefV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if current_runtime_scope.world_id() != self.commit.world_id() {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        self.current_runtime_scope = current_runtime_scope;
        Ok(self)
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
    pub const fn current_character_world_eligibility(
        self,
    ) -> Option<CharacterWorldEligibilityClaimV1> {
        self.current_character_world_eligibility
    }

    #[must_use]
    pub const fn current_runtime_scope(self) -> RuntimeScopeRefV1 {
        self.current_runtime_scope
    }

    #[must_use]
    pub const fn current_scope_generation(self) -> ScopeOwnershipGeneration {
        self.current_scope_generation
    }

    #[must_use]
    pub const fn current_control_loss_epoch(self) -> Option<ControlLossEpochRefV1> {
        self.current_control_loss_epoch
    }

    #[must_use]
    pub const fn current_original_grace_deadline(self) -> Option<i64> {
        self.current_original_grace_deadline
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalGameSessionReplacementAuthorizationV1 {
    account_id: String,
    character_id: CharacterId,
    world_id: WorldId,
    predecessor_game_session_id: GameSessionId,
    predecessor_connection_generation: ConnectionGeneration,
    predecessor_character_lease_generation: u64,
    predecessor_current_scope_ownership_generation: ScopeOwnershipGeneration,
    predecessor_control_loss_epoch: ControlLossEpochRefV1,
    predecessor_original_grace_deadline: i64,
    candidate_game_session_id: GameSessionId,
    candidate_runtime_scope: RuntimeScopeRefV1,
}

impl TerminalGameSessionReplacementAuthorizationV1 {
    pub fn from_current_authority<T: Copy + Eq>(
        account_id: &str,
        current_account_presence: Option<&AccountPresenceClaimV1>,
        predecessor_game_session_id: GameSessionId,
        candidate_game_session_id: GameSessionId,
        snapshot: GameSessionAuthoritySnapshot<T>,
        candidate: &ReconnectDurabilityRecordV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !canonical_uuid(account_id)
            || current_account_presence.is_none_or(|presence| {
                presence.account_id() != account_id
                    || presence.character_id() != snapshot.commit().character_id()
                    || presence.character_id() != candidate.identity().character_id()
            })
            || predecessor_game_session_id == candidate_game_session_id
            || snapshot.session_state() != GameSessionState::Terminal
            || snapshot.current_transport().is_some()
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }

        validate_current_authority(predecessor_game_session_id, snapshot)
            .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;

        let committed = snapshot.commit();
        let current_lease = snapshot.current_character_lease();
        let Some(current_control_loss_epoch) = snapshot.current_control_loss_epoch() else {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        };
        let Some(current_original_grace_deadline) = snapshot.current_original_grace_deadline()
        else {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        };
        let identity = candidate.identity();
        let candidate_authority = candidate.authority();
        let candidate_continuity = candidate.continuity();

        if committed.game_session_id() != predecessor_game_session_id
            || identity.game_session_id() != candidate_game_session_id
            || identity.account_id() != account_id
            || identity.character_id() != committed.character_id()
            || identity.world_id() != committed.world_id()
            || identity.runtime_scope() != snapshot.current_runtime_scope()
            || candidate.connection().predecessor() != snapshot.current_connection_generation()
            || current_lease.character_id() != committed.character_id()
            || candidate_authority.character_lease_generation() != current_lease.generation()
            || candidate_authority.scope_ownership_generation()
                != snapshot.current_scope_generation()
            || candidate_continuity.control_loss_epoch() != current_control_loss_epoch
            || candidate_continuity.original_grace_deadline() != current_original_grace_deadline
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }

        Ok(Self {
            account_id: account_id.to_owned(),
            character_id: committed.character_id(),
            world_id: committed.world_id(),
            predecessor_game_session_id,
            predecessor_connection_generation: snapshot.current_connection_generation(),
            predecessor_character_lease_generation: current_lease.generation(),
            predecessor_current_scope_ownership_generation: snapshot.current_scope_generation(),
            predecessor_control_loss_epoch: current_control_loss_epoch,
            predecessor_original_grace_deadline: current_original_grace_deadline,
            candidate_game_session_id,
            candidate_runtime_scope: identity.runtime_scope(),
        })
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
    pub const fn predecessor_game_session_id(&self) -> GameSessionId {
        self.predecessor_game_session_id
    }

    #[must_use]
    pub const fn predecessor_connection_generation(&self) -> ConnectionGeneration {
        self.predecessor_connection_generation
    }

    #[must_use]
    pub const fn predecessor_character_lease_generation(&self) -> u64 {
        self.predecessor_character_lease_generation
    }

    #[must_use]
    pub const fn predecessor_current_scope_ownership_generation(
        &self,
    ) -> ScopeOwnershipGeneration {
        self.predecessor_current_scope_ownership_generation
    }

    #[must_use]
    pub const fn predecessor_control_loss_epoch(&self) -> ControlLossEpochRefV1 {
        self.predecessor_control_loss_epoch
    }

    #[must_use]
    pub const fn predecessor_original_grace_deadline(&self) -> i64 {
        self.predecessor_original_grace_deadline
    }

    #[must_use]
    pub const fn candidate_game_session_id(&self) -> GameSessionId {
        self.candidate_game_session_id
    }

    #[must_use]
    pub const fn candidate_runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.candidate_runtime_scope
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
    if snapshot.current_character_world_eligibility()
        != Some(CharacterWorldEligibilityClaimV1::new(
            committed.character_id(),
            committed.world_id(),
        ))
    {
        return Err(AdmissionError::ReconciliationUnavailable);
    }

    let lease = snapshot.current_character_lease();
    if lease.character_id() != committed.character_id() {
        return Err(AdmissionError::StaleLease);
    }
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
    Channel { world_id: WorldId, channel_id: ChannelId },
    Instance { world_id: WorldId, instance_id: [u8; 16] },
}

impl RuntimeScopeRefV1 {
    #[must_use]
    pub const fn channel(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self::Channel { world_id, channel_id }
    }

    pub fn instance(world_id: WorldId, instance_id: [u8; 16]) -> Result<Self, ReconnectDurabilityErrorV1> {
        if instance_id.iter().all(|byte| *byte == 0)
            || instance_id[6] >> 4 != 7
            || instance_id[8] >> 6 != 2
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self::Instance { world_id, instance_id })
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
    pub const fn game_session_id(&self) -> GameSessionId { self.game_session_id }
    #[must_use]
    pub const fn reconnect_attempt_ref(&self) -> ReconnectAttemptRef { self.reconnect_attempt_ref }
    #[must_use]
    pub fn account_id(&self) -> &str { &self.account_id }
    #[must_use]
    pub const fn character_id(&self) -> CharacterId { self.character_id }
    #[must_use]
    pub const fn world_id(&self) -> WorldId { self.world_id }
    #[must_use]
    pub const fn runtime_scope(&self) -> RuntimeScopeRefV1 { self.runtime_scope }
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
        Ok(Self { predecessor, candidate, transport_ref })
    }
    #[must_use]
    pub const fn predecessor(self) -> ConnectionGeneration { self.predecessor }
    #[must_use]
    pub const fn candidate(self) -> ConnectionGeneration { self.candidate }
    #[must_use]
    pub const fn transport_ref(self) -> AuthenticatedTransportRefV1 { self.transport_ref }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectCandidateBindingV1 {
    game_session_id: GameSessionId,
    reconnect_attempt_ref: ReconnectAttemptRef,
    connection_generation: ConnectionGeneration,
    transport_ref: AuthenticatedTransportRefV1,
    prepared_deadline: i64,
}

impl ReconnectCandidateBindingV1 {
    pub fn new(
        game_session_id: GameSessionId,
        reconnect_attempt_ref: ReconnectAttemptRef,
        connection_generation: ConnectionGeneration,
        transport_ref: AuthenticatedTransportRefV1,
        prepared_deadline: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if prepared_deadline <= 0 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            game_session_id,
            reconnect_attempt_ref,
            connection_generation,
            transport_ref,
            prepared_deadline,
        })
    }

    fn expected_binding_from_record(
        record: &ReconnectDurabilityRecordV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        Self::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )
    }

    #[cfg(test)]
    pub fn from_record(
        record: &ReconnectDurabilityRecordV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        Self::expected_binding_from_record(record)
    }

    #[must_use]
    pub const fn game_session_id(self) -> GameSessionId {
        self.game_session_id
    }

    #[must_use]
    pub const fn reconnect_attempt_ref(self) -> ReconnectAttemptRef {
        self.reconnect_attempt_ref
    }

    #[must_use]
    pub const fn connection_generation(self) -> ConnectionGeneration {
        self.connection_generation
    }

    #[must_use]
    pub const fn transport_ref(self) -> AuthenticatedTransportRefV1 {
        self.transport_ref
    }

    #[must_use]
    pub const fn prepared_deadline(self) -> i64 {
        self.prepared_deadline
    }

    #[must_use]
    const fn is_live_at(self, observed_at: i64) -> bool {
        observed_at >= 0 && observed_at <= self.prepared_deadline
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
        Ok(Self { character_lease_generation, scope_ownership_generation })
    }
    #[must_use]
    pub const fn character_lease_generation(self) -> u64 { self.character_lease_generation }
    #[must_use]
    pub const fn scope_ownership_generation(self) -> ScopeOwnershipGeneration { self.scope_ownership_generation }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionEntitlementV1 {
    Unused,
    Fenced { generation: u64 },
}

impl ProtectionEntitlementV1 {
    #[must_use]
    pub const fn unused() -> Self { Self::Unused }
    pub fn fenced(generation: u64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if generation == 0 { return Err(ReconnectDurabilityErrorV1::InvalidRecord); }
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
        if prepared_deadline <= 0 || original_grace_deadline <= 0 || prepared_deadline > original_grace_deadline {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self { control_loss_epoch, original_grace_deadline, prepared_deadline, protection_entitlement })
    }
    #[must_use]
    pub const fn control_loss_epoch(self) -> ControlLossEpochRefV1 { self.control_loss_epoch }
    #[must_use]
    pub const fn original_grace_deadline(self) -> i64 { self.original_grace_deadline }
    #[must_use]
    pub const fn prepared_deadline(self) -> i64 { self.prepared_deadline }
    #[must_use]
    pub const fn protection_entitlement(self) -> ProtectionEntitlementV1 { self.protection_entitlement }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectProofV1 {
    FastReconnect { reconnect_proof_generation: u64 },
    ReauthenticatedRecovery { recovery_grant_nonce: [u8; 32] },
}
impl ReconnectProofV1 {
    fn validate(&self) -> bool {
        match self {
            Self::FastReconnect { reconnect_proof_generation } => *reconnect_proof_generation != 0,
            Self::ReauthenticatedRecovery { recovery_grant_nonce } => recovery_grant_nonce.iter().any(|byte| *byte != 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingCommandDispositionV1 { PendingOriginal, TerminalOutcomeRetained }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCommandReconciliationV1 { command_id: CommandId, disposition: PendingCommandDispositionV1 }
impl PendingCommandReconciliationV1 {
    #[must_use]
    pub const fn new(command_id: CommandId, disposition: PendingCommandDispositionV1) -> Self { Self { command_id, disposition } }
    #[must_use]
    pub const fn command_id(self) -> CommandId { self.command_id }
    #[must_use]
    pub const fn disposition(self) -> PendingCommandDispositionV1 { self.disposition }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateDomainRevisionV1 { domain_id: u32, revision: u64 }
impl StateDomainRevisionV1 {
    pub fn new(domain_id: u32, revision: u64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if domain_id == 0 { return Err(ReconnectDurabilityErrorV1::InvalidRecord); }
        Ok(Self { domain_id, revision })
    }
    #[must_use]
    pub const fn domain_id(self) -> u32 { self.domain_id }
    #[must_use]
    pub const fn revision(self) -> u64 { self.revision }
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
            || pending.iter().any(|item| item.command_id().get() >= next_command_id.get())
            || pending.windows(2).any(|pair| pair[0].command_id() >= pair[1].command_id())
            || domain_revisions.windows(2).any(|pair| pair[0].domain_id() >= pair[1].domain_id())
        { return Err(ReconnectDurabilityErrorV1::InvalidRecord); }
        Ok(Self { next_command_id, pending, server_sequence, domain_revisions })
    }
    #[must_use]
    pub const fn next_command_id(&self) -> CommandId { self.next_command_id }
    #[must_use]
    pub fn pending(&self) -> &[PendingCommandReconciliationV1] { &self.pending }
    #[must_use]
    pub const fn server_sequence(&self) -> u64 { self.server_sequence }
    #[must_use]
    pub fn domain_revisions(&self) -> &[StateDomainRevisionV1] { &self.domain_revisions }
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
    pub fn new(authority: &str, purpose: &str, scope: &str, source_revision: &str, decision_identity: &str, source_observed_at: i64) -> Result<Self, ReconnectDurabilityErrorV1> {
        if source_observed_at < 0 || ![authority, purpose, scope, source_revision, decision_identity].iter().all(|value| non_empty_visible_ascii(value)) {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self { authority: authority.to_owned(), purpose: purpose.to_owned(), scope: scope.to_owned(), source_revision: source_revision.to_owned(), decision_identity: decision_identity.to_owned(), source_observed_at })
    }
    #[must_use]
    pub fn authority(&self) -> &str { &self.authority }
    #[must_use]
    pub fn purpose(&self) -> &str { &self.purpose }
    #[must_use]
    pub fn scope(&self) -> &str { &self.scope }
    #[must_use]
    pub fn source_revision(&self) -> &str { &self.source_revision }
    #[must_use]
    pub fn decision_identity(&self) -> &str { &self.decision_identity }
    #[must_use]
    pub const fn source_observed_at(&self) -> i64 { self.source_observed_at }
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
    pub fn new(protocol_major: u32, transport_profile: u32, ruleset_revision: &str, content_revision: &str, map_revision: &str, world_policy_revision: &str, account_security_generation: u64, platform_security_evidence: AuthorityEvidenceFenceV1, proof_trust_evidence: AuthorityEvidenceFenceV1, credential_expiration: Option<i64>) -> Result<Self, ReconnectDurabilityErrorV1> {
        if protocol_major == 0 || transport_profile == 0 || account_security_generation == 0
            || ![ruleset_revision, content_revision, map_revision, world_policy_revision].iter().all(|value| non_empty_visible_ascii(value))
            || credential_expiration.is_some_and(|value| value <= 0)
        { return Err(ReconnectDurabilityErrorV1::InvalidRecord); }
        Ok(Self { protocol_major, transport_profile, ruleset_revision: ruleset_revision.to_owned(), content_revision: content_revision.to_owned(), map_revision: map_revision.to_owned(), world_policy_revision: world_policy_revision.to_owned(), account_security_generation, platform_security_evidence, proof_trust_evidence, credential_expiration })
    }
    #[must_use]
    pub const fn protocol_major(&self) -> u32 { self.protocol_major }
    #[must_use]
    pub const fn transport_profile(&self) -> u32 { self.transport_profile }
    #[must_use]
    pub fn ruleset_revision(&self) -> &str { &self.ruleset_revision }
    #[must_use]
    pub fn content_revision(&self) -> &str { &self.content_revision }
    #[must_use]
    pub fn map_revision(&self) -> &str { &self.map_revision }
    #[must_use]
    pub fn world_policy_revision(&self) -> &str { &self.world_policy_revision }
    #[must_use]
    pub const fn account_security_generation(&self) -> u64 { self.account_security_generation }
    #[must_use]
    pub const fn credential_expiration(&self) -> Option<i64> { self.credential_expiration }
    #[must_use]
    pub const fn platform_security_evidence(&self) -> &AuthorityEvidenceFenceV1 { &self.platform_security_evidence }
    #[must_use]
    pub const fn proof_trust_evidence(&self) -> &AuthorityEvidenceFenceV1 { &self.proof_trust_evidence }
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
    pub fn new(identity: ReconnectIdentityV1, connection: ReconnectConnectionFenceV1, authority: ReconnectAuthorityFenceV1, continuity: ReconnectContinuityV1, proof: ReconnectProofV1, fnd02: Fnd02ReconciliationFenceV1, compatibility: ReconnectCompatibilityEvidenceV1) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !proof.validate() { return Err(ReconnectDurabilityErrorV1::InvalidRecord); }
        Ok(Self { identity, connection, authority, continuity, proof, fnd02, compatibility })
    }
    #[must_use]
    pub const fn version(&self) -> u16 { 1 }
    #[must_use]
    pub const fn identity(&self) -> &ReconnectIdentityV1 { &self.identity }
    #[must_use]
    pub const fn connection(&self) -> ReconnectConnectionFenceV1 { self.connection }
    #[must_use]
    pub const fn authority(&self) -> ReconnectAuthorityFenceV1 { self.authority }
    #[must_use]
    pub const fn continuity(&self) -> ReconnectContinuityV1 { self.continuity }
    #[must_use]
    pub const fn proof(&self) -> &ReconnectProofV1 { &self.proof }
    #[must_use]
    pub const fn fnd02(&self) -> &Fnd02ReconciliationFenceV1 { &self.fnd02 }
    #[must_use]
    pub const fn compatibility(&self) -> &ReconnectCompatibilityEvidenceV1 { &self.compatibility }
    fn authorization_deadline(&self) -> Result<i64, ReconnectDurabilityErrorV1> {
        let compatibility = self.compatibility();
        let platform_deadline = compatibility.platform_security_evidence().source_observed_at().checked_add(EVIDENCE_FRESHNESS_SECONDS_V1).ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        let trust_deadline = compatibility.proof_trust_evidence().source_observed_at().checked_add(EVIDENCE_FRESHNESS_SECONDS_V1).ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        let mut deadline = self.continuity().prepared_deadline().min(self.continuity().original_grace_deadline()).min(platform_deadline).min(trust_deadline);
        if let Some(credential_expiration) = compatibility.credential_expiration() { deadline = deadline.min(credential_expiration); }
        Ok(deadline)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAttemptReservationV1 { New, Existing }
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
enum ReconnectAttemptStateV1 { Reserved, Prepared, CollisionTerminal, Terminal }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReconnectAttemptEntryV1 { attempt: ReconnectAttemptRef, transport_ref: AuthenticatedTransportRefV1, state: ReconnectAttemptStateV1 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectAttemptBudgetV1 { control_loss_epoch: ControlLossEpochRefV1, entries: Vec<ReconnectAttemptEntryV1> }
impl ReconnectAttemptBudgetV1 {
    #[must_use]
    pub const fn new(control_loss_epoch: ControlLossEpochRefV1) -> Self { Self { control_loss_epoch, entries: Vec::new() } }
    #[must_use]
    pub const fn control_loss_epoch(&self) -> ControlLossEpochRefV1 { self.control_loss_epoch }
    #[must_use]
    pub fn distinct_attempts(&self) -> usize { self.entries.len() }
    pub fn reserve(&mut self, attempt: ReconnectAttemptRef, transport_ref: AuthenticatedTransportRefV1) -> Result<ReconnectAttemptReservationV1, ReconnectDurabilityErrorV1> {
        if let Some(entry) = self.entries.iter().find(|entry| entry.attempt == attempt) {
            return if entry.transport_ref == transport_ref { Ok(ReconnectAttemptReservationV1::Existing) } else { Err(ReconnectDurabilityErrorV1::IdempotencyConflict) };
        }
        if self.entries.len() >= RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1 { return Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded); }
        self.entries.push(ReconnectAttemptEntryV1 { attempt, transport_ref, state: ReconnectAttemptStateV1::Reserved });
        Ok(ReconnectAttemptReservationV1::New)
    }
    pub fn accept_prepare_completion(&mut self, attempt: ReconnectAttemptRef, transport_ref: AuthenticatedTransportRefV1, disposition: ReconnectPrepareDispositionV1) -> Result<(), ReconnectDurabilityErrorV1> {
        let index = self.entries.iter().position(|entry| entry.attempt == attempt).ok_or(ReconnectDurabilityErrorV1::CompletionMismatch)?;
        if self.entries[index].transport_ref != transport_ref { return Err(ReconnectDurabilityErrorV1::IdempotencyConflict); }
        match disposition {
            ReconnectPrepareDispositionV1::Prepared | ReconnectPrepareDispositionV1::ExistingPrepared => {
                if self.entries.iter().enumerate().any(|(other, entry)| other != index && entry.state == ReconnectAttemptStateV1::Prepared) { return Err(ReconnectDurabilityErrorV1::ConcurrentPrepared); }
                self.entries[index].state = ReconnectAttemptStateV1::Prepared;
            }
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision => self.entries[index].state = ReconnectAttemptStateV1::CollisionTerminal,
            ReconnectPrepareDispositionV1::Unavailable | ReconnectPrepareDispositionV1::Ambiguous => {}
            _ => self.entries[index].state = ReconnectAttemptStateV1::Terminal,
        }
        Ok(())
    }
    #[must_use]
    pub fn replacement_allowed_after_collision(&self, attempt: ReconnectAttemptRef) -> bool {
        self.entries.len() < RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1
            && !self.entries.iter().any(|entry| entry.state == ReconnectAttemptStateV1::Prepared)
            && self.entries.iter().any(|entry| entry.attempt == attempt && entry.state == ReconnectAttemptStateV1::CollisionTerminal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareRequestV1 { record: Box<ReconnectDurabilityRecordV1> }
impl ReconnectPrepareRequestV1 {
    #[must_use]
    pub fn record(&self) -> &ReconnectDurabilityRecordV1 { &self.record }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareCompletionV1 { request: ReconnectPrepareRequestV1, disposition: ReconnectPrepareDispositionV1 }
impl ReconnectPrepareCompletionV1 {
    #[must_use]
    pub fn for_request(request: &ReconnectPrepareRequestV1, disposition: ReconnectPrepareDispositionV1) -> Self { Self { request: request.clone(), disposition } }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectPrepareActionV1 {
    RetrySameRequest(ReconnectPrepareRequestV1),
    AwaitFinalRevalidation,
    ReconcileSameAttempt,
    Terminal(ReconnectPrepareDispositionV1),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurabilityPhaseV1 { PendingPrepare, AwaitFinalRevalidation, PendingCommit, ReconciliationRequired, Terminal, Completed }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountPresenceClaimV1 {
    account_id: String,
    character_id: CharacterId,
}

impl AccountPresenceClaimV1 {
    pub fn new(
        account_id: &str,
        character_id: CharacterId,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !canonical_uuid(account_id) {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            account_id: account_id.to_owned(),
            character_id,
        })
    }

    fn expected_from_identity(
        identity: &ReconnectIdentityV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        Self::new(identity.account_id(), identity.character_id())
    }

    #[cfg(test)]
    pub fn from_identity(
        identity: &ReconnectIdentityV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        Self::expected_from_identity(identity)
    }

    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    #[must_use]
    pub const fn character_id(&self) -> CharacterId {
        self.character_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCurrentAuthorityV1 {
    identity: ReconnectIdentityV1,
    current_account_presence: Option<AccountPresenceClaimV1>,
    current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
    current_candidate: Option<ReconnectCandidateBindingV1>,
    current_runtime_scope: RuntimeScopeRefV1,
    predecessor: ConnectionGeneration,
    authority: ReconnectAuthorityFenceV1,
    continuity_epoch: ControlLossEpochRefV1,
    original_grace_deadline: i64,
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
    #[allow(clippy::too_many_arguments)]
    pub fn from_current_facts(
        record: &ReconnectDurabilityRecordV1,
        current_account_presence: Option<AccountPresenceClaimV1>,
        current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
        current_candidate: Option<ReconnectCandidateBindingV1>,
        current_runtime_scope: RuntimeScopeRefV1,
        predecessor: ConnectionGeneration,
        authority: ReconnectAuthorityFenceV1,
        continuity_epoch: ControlLossEpochRefV1,
        original_grace_deadline: i64,
        proof: ReconnectProofV1,
        fnd02: Fnd02ReconciliationFenceV1,
        compatibility: ReconnectCompatibilityEvidenceV1,
        session_state: GameSessionState,
        current_controller_present: bool,
        observed_at: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if observed_at < 0
            || original_grace_deadline <= 0
            || current_runtime_scope.world_id() != record.identity().world_id()
            || !proof.validate()
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self {
            identity: record.identity().clone(),
            current_account_presence,
            current_character_world_eligibility,
            current_candidate,
            current_runtime_scope,
            predecessor,
            authority,
            continuity_epoch,
            original_grace_deadline,
            proof,
            fnd02,
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
            session_state,
            current_controller_present,
            observed_at,
        })
    }

    #[cfg(test)]
    pub fn from_record(
        record: &ReconnectDurabilityRecordV1,
        observed_at: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        Self::from_current_facts(
            record,
            Some(AccountPresenceClaimV1::from_identity(record.identity())?),
            Some(CharacterWorldEligibilityClaimV1::from_identity(record.identity())),
            Some(ReconnectCandidateBindingV1::expected_binding_from_record(record)?),
            record.identity().runtime_scope(),
            record.connection().predecessor(),
            record.authority(),
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline(),
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            observed_at,
        )
    }

    pub fn with_current_runtime_scope(
        mut self,
        current_runtime_scope: RuntimeScopeRefV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if current_runtime_scope.world_id() != self.identity.world_id() {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        self.current_runtime_scope = current_runtime_scope;
        Ok(self)
    }

    #[must_use]
    pub const fn current_runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.current_runtime_scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectCommitAuthorizationV1 { authorization_deadline: i64 }
impl ReconnectCommitAuthorizationV1 {
    #[must_use]
    pub const fn authorization_deadline(self) -> i64 { self.authorization_deadline }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCommitRequestV1 { record: Box<ReconnectDurabilityRecordV1>, authorization: ReconnectCommitAuthorizationV1 }
impl ReconnectCommitRequestV1 {
    #[must_use]
    pub fn record(&self) -> &ReconnectDurabilityRecordV1 { &self.record }
    #[must_use]
    pub const fn authorization(&self) -> ReconnectCommitAuthorizationV1 { self.authorization }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectCommitDispositionV1 { Committed, Unavailable, Ambiguous, RejectedStaleAuthority, ExistingTerminal, IdempotencyConflict }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectCommitCompletionV1 { request: ReconnectCommitRequestV1, disposition: ReconnectCommitDispositionV1 }
impl ReconnectCommitCompletionV1 {
    #[must_use]
    pub fn for_request(request: &ReconnectCommitRequestV1, disposition: ReconnectCommitDispositionV1) -> Self { Self { request: request.clone(), disposition } }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectCommitActionV1 { RetrySameRequest(ReconnectCommitRequestV1), ReconcileSameAttempt, Terminal(ReconnectCommitDispositionV1) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableReconnectStateV1 { Prepared, Committed, Terminal }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurableReconciliationSnapshotV1 {
    record: ReconnectDurabilityRecordV1,
    durable_state: DurableReconnectStateV1,
    current_generation: Option<ConnectionGeneration>,
    current_transport_ref: Option<AuthenticatedTransportRefV1>,
}
impl ReconnectDurableReconciliationSnapshotV1 {
    #[must_use]
    pub fn committed(record: ReconnectDurabilityRecordV1) -> Self { Self { current_generation: Some(record.connection().candidate()), current_transport_ref: Some(record.connection().transport_ref()), record, durable_state: DurableReconnectStateV1::Committed } }
    #[must_use]
    pub fn prepared(record: ReconnectDurabilityRecordV1) -> Self { Self { current_generation: None, current_transport_ref: None, record, durable_state: DurableReconnectStateV1::Prepared } }
    #[must_use]
    pub fn terminal(record: ReconnectDurabilityRecordV1) -> Self { Self { current_generation: None, current_transport_ref: None, record, durable_state: DurableReconnectStateV1::Terminal } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectProjectionDecisionV1 {
    InstallController { generation: ConnectionGeneration, transport_ref: AuthenticatedTransportRefV1 },
    AwaitFinalRevalidation,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurableTerminalDispositionV1 {
    TransportRefCollision,
    ConcurrentPrepared,
    StaleAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurableOutcomeV2 {
    Prepared,
    Committed {
        current_generation: ConnectionGeneration,
        current_transport_ref: AuthenticatedTransportRefV1,
    },
    Terminal {
        disposition: ReconnectDurableTerminalDispositionV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurableReconciliationSnapshotV2 {
    record: ReconnectDurabilityRecordV1,
    outcome: ReconnectDurableOutcomeV2,
}

impl ReconnectDurableReconciliationSnapshotV2 {
    #[must_use]
    pub fn new(record: ReconnectDurabilityRecordV1, outcome: ReconnectDurableOutcomeV2) -> Self {
        Self { record, outcome }
    }

    #[must_use]
    pub const fn record(&self) -> &ReconnectDurabilityRecordV1 {
        &self.record
    }

    #[must_use]
    pub const fn outcome(&self) -> ReconnectDurableOutcomeV2 {
        self.outcome
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareRequestV2 {
    record: Box<ReconnectDurabilityRecordV1>,
    terminal_replacement: Option<TerminalGameSessionReplacementAuthorizationV1>,
}

impl ReconnectPrepareRequestV2 {
    #[must_use]
    pub fn record(&self) -> &ReconnectDurabilityRecordV1 {
        &self.record
    }

    #[must_use]
    pub const fn terminal_replacement(
        &self,
    ) -> Option<&TerminalGameSessionReplacementAuthorizationV1> {
        self.terminal_replacement.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectPrepareDispositionV2 {
    Prepared,
    ExistingPrepared,
    RejectedTransportRefCollision,
    RejectedConcurrentPrepared,
    RejectedStaleAuthority,
    AttemptCapacityExceeded,
    ExistingTerminal {
        disposition: ReconnectDurableTerminalDispositionV1,
    },
    Unavailable,
    Ambiguous,
    IdempotencyConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareCompletionV2 {
    request: ReconnectPrepareRequestV2,
    disposition: ReconnectPrepareDispositionV2,
}

impl ReconnectPrepareCompletionV2 {
    #[must_use]
    pub fn for_request(
        request: &ReconnectPrepareRequestV2,
        disposition: ReconnectPrepareDispositionV2,
    ) -> Self {
        Self {
            request: request.clone(),
            disposition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectPrepareActionV2 {
    RetrySameRequest(ReconnectPrepareRequestV2),
    AwaitFinalRevalidation,
    ReconcileSameAttempt,
    Terminal(ReconnectPrepareDispositionV2),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectProjectionDecisionV2 {
    InstallController {
        generation: ConnectionGeneration,
        transport_ref: AuthenticatedTransportRefV1,
    },
    AwaitFinalRevalidation,
    Terminal {
        disposition: ReconnectDurableTerminalDispositionV1,
    },
}

impl ReconnectProjectionDecisionV2 {
    #[must_use]
    pub const fn terminal_disposition(self) -> Option<ReconnectDurableTerminalDispositionV1> {
        match self {
            Self::Terminal { disposition } => Some(disposition),
            Self::InstallController { .. } | Self::AwaitFinalRevalidation => None,
        }
    }
}

fn v1_budget_disposition_for_terminal(
    disposition: ReconnectDurableTerminalDispositionV1,
) -> ReconnectPrepareDispositionV1 {
    match disposition {
        ReconnectDurableTerminalDispositionV1::TransportRefCollision => {
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision
        }
        ReconnectDurableTerminalDispositionV1::ConcurrentPrepared => {
            ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
        }
        ReconnectDurableTerminalDispositionV1::StaleAuthority => {
            ReconnectPrepareDispositionV1::RejectedStaleAuthority
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurabilityFlowV2 {
    record: ReconnectDurabilityRecordV1,
    prepare_request: ReconnectPrepareRequestV2,
    phase: ReconnectDurabilityPhaseV1,
}

impl ReconnectDurabilityFlowV2 {
    #[must_use]
    pub fn begin(
        record: ReconnectDurabilityRecordV1,
        terminal_replacement: Option<TerminalGameSessionReplacementAuthorizationV1>,
    ) -> (Self, ReconnectPrepareRequestV2) {
        let prepare_request = ReconnectPrepareRequestV2 {
            record: Box::new(record.clone()),
            terminal_replacement,
        };
        (
            Self {
                record,
                prepare_request: prepare_request.clone(),
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
        completion: ReconnectPrepareCompletionV2,
        budget: &mut ReconnectAttemptBudgetV1,
    ) -> Result<ReconnectPrepareActionV2, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::PendingPrepare {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if completion.request != self.prepare_request {
            return Err(ReconnectDurabilityErrorV1::CompletionMismatch);
        }

        let attempt = self.record.identity().reconnect_attempt_ref();
        let transport_ref = self.record.connection().transport_ref();
        match completion.disposition {
            ReconnectPrepareDispositionV2::Prepared => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    ReconnectPrepareDispositionV1::Prepared,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation;
                Ok(ReconnectPrepareActionV2::AwaitFinalRevalidation)
            }
            ReconnectPrepareDispositionV2::ExistingPrepared => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    ReconnectPrepareDispositionV1::ExistingPrepared,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation;
                Ok(ReconnectPrepareActionV2::AwaitFinalRevalidation)
            }
            ReconnectPrepareDispositionV2::Unavailable => Ok(
                ReconnectPrepareActionV2::RetrySameRequest(self.prepare_request.clone()),
            ),
            ReconnectPrepareDispositionV2::Ambiguous => {
                self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired;
                Ok(ReconnectPrepareActionV2::ReconcileSameAttempt)
            }
            ReconnectPrepareDispositionV2::RejectedTransportRefCollision => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    ReconnectPrepareDispositionV1::RejectedTransportRefCollision,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV2::Terminal(
                    ReconnectPrepareDispositionV2::RejectedTransportRefCollision,
                ))
            }
            ReconnectPrepareDispositionV2::RejectedConcurrentPrepared => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    ReconnectPrepareDispositionV1::RejectedConcurrentPrepared,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV2::Terminal(
                    ReconnectPrepareDispositionV2::RejectedConcurrentPrepared,
                ))
            }
            ReconnectPrepareDispositionV2::RejectedStaleAuthority => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV2::Terminal(
                    ReconnectPrepareDispositionV2::RejectedStaleAuthority,
                ))
            }
            ReconnectPrepareDispositionV2::ExistingTerminal { disposition } => {
                budget.accept_prepare_completion(
                    attempt,
                    transport_ref,
                    v1_budget_disposition_for_terminal(disposition),
                )?;
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV2::Terminal(
                    ReconnectPrepareDispositionV2::ExistingTerminal { disposition },
                ))
            }
            terminal @ (ReconnectPrepareDispositionV2::AttemptCapacityExceeded
            | ReconnectPrepareDispositionV2::IdempotencyConflict) => {
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectPrepareActionV2::Terminal(terminal))
            }
        }
    }

    pub fn accept_reconciliation(
        &mut self,
        snapshot: ReconnectDurableReconciliationSnapshotV2,
        current: ReconnectCurrentAuthorityV1,
        budget: &mut ReconnectAttemptBudgetV1,
    ) -> Result<ReconnectProjectionDecisionV2, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if snapshot.record != self.record {
            return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch);
        }

        match snapshot.outcome {
            ReconnectDurableOutcomeV2::Prepared => {
                budget.accept_prepare_completion(
                    self.record.identity().reconnect_attempt_ref(),
                    self.record.connection().transport_ref(),
                    ReconnectPrepareDispositionV1::ExistingPrepared,
                )?;
                self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation;
                Ok(ReconnectProjectionDecisionV2::AwaitFinalRevalidation)
            }
            ReconnectDurableOutcomeV2::Committed {
                current_generation,
                current_transport_ref,
            } => {
                if current_generation != self.record.connection().candidate()
                    || current_transport_ref != self.record.connection().transport_ref()
                    || current.observed_at > self.record.authorization_deadline()?
                    || !current_authority_matches_record(&self.record, &current)?
                {
                    return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch);
                }
                self.phase = ReconnectDurabilityPhaseV1::Completed;
                Ok(ReconnectProjectionDecisionV2::InstallController {
                    generation: current_generation,
                    transport_ref: current_transport_ref,
                })
            }
            ReconnectDurableOutcomeV2::Terminal { disposition } => {
                budget.accept_prepare_completion(
                    self.record.identity().reconnect_attempt_ref(),
                    self.record.connection().transport_ref(),
                    v1_budget_disposition_for_terminal(disposition),
                )?;
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectProjectionDecisionV2::Terminal { disposition })
            }
        }
    }
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
        let prepare_request = ReconnectPrepareRequestV1 { record: Box::new(record.clone()) };
        (Self { record, prepare_request: prepare_request.clone(), commit_request: None, phase: ReconnectDurabilityPhaseV1::PendingPrepare }, prepare_request)
    }
    #[must_use]
    pub const fn phase(&self) -> ReconnectDurabilityPhaseV1 { self.phase }
    pub fn accept_prepare_completion(&mut self, completion: ReconnectPrepareCompletionV1) -> Result<ReconnectPrepareActionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::PendingPrepare { return Err(ReconnectDurabilityErrorV1::InvalidPhase); }
        if completion.request != self.prepare_request { return Err(ReconnectDurabilityErrorV1::CompletionMismatch); }
        match completion.disposition {
            ReconnectPrepareDispositionV1::Prepared | ReconnectPrepareDispositionV1::ExistingPrepared => { self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation; Ok(ReconnectPrepareActionV1::AwaitFinalRevalidation) }
            ReconnectPrepareDispositionV1::Unavailable => Ok(ReconnectPrepareActionV1::RetrySameRequest(self.prepare_request.clone())),
            ReconnectPrepareDispositionV1::Ambiguous | ReconnectPrepareDispositionV1::ExistingTerminal => { self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired; Ok(ReconnectPrepareActionV1::ReconcileSameAttempt) }
            terminal => { self.phase = ReconnectDurabilityPhaseV1::Terminal; Ok(ReconnectPrepareActionV1::Terminal(terminal)) }
        }
    }
    pub fn authorize_commit(&mut self, current: ReconnectCurrentAuthorityV1, now: i64) -> Result<ReconnectCommitRequestV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::AwaitFinalRevalidation { return Err(ReconnectDurabilityErrorV1::InvalidPhase); }
        if !current_authority_matches_record(&self.record, &current)? || !authenticated_evidence_observed_by(&self.record, now) { self.phase = ReconnectDurabilityPhaseV1::Terminal; return Err(ReconnectDurabilityErrorV1::StaleAuthority); }
        let deadline = self.record.authorization_deadline()?;
        if now > deadline || current.observed_at > deadline { self.phase = ReconnectDurabilityPhaseV1::Terminal; return Err(ReconnectDurabilityErrorV1::DeadlineExpired); }
        let request = ReconnectCommitRequestV1 { record: Box::new(self.record.clone()), authorization: ReconnectCommitAuthorizationV1 { authorization_deadline: deadline } };
        self.commit_request = Some(request.clone()); self.phase = ReconnectDurabilityPhaseV1::PendingCommit; Ok(request)
    }
    pub fn accept_commit_completion(&mut self, completion: ReconnectCommitCompletionV1) -> Result<ReconnectCommitActionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::PendingCommit { return Err(ReconnectDurabilityErrorV1::InvalidPhase); }
        if self.commit_request.as_ref() != Some(&completion.request) { return Err(ReconnectDurabilityErrorV1::CompletionMismatch); }
        match completion.disposition {
            ReconnectCommitDispositionV1::Unavailable => Ok(ReconnectCommitActionV1::RetrySameRequest(completion.request)),
            ReconnectCommitDispositionV1::Committed | ReconnectCommitDispositionV1::Ambiguous => { self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired; Ok(ReconnectCommitActionV1::ReconcileSameAttempt) }
            terminal => { self.phase = ReconnectDurabilityPhaseV1::Terminal; Ok(ReconnectCommitActionV1::Terminal(terminal)) }
        }
    }
    pub fn accept_reconciliation(&mut self, snapshot: ReconnectDurableReconciliationSnapshotV1, current: ReconnectCurrentAuthorityV1) -> Result<ReconnectProjectionDecisionV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::ReconciliationRequired { return Err(ReconnectDurabilityErrorV1::InvalidPhase); }
        if snapshot.record != self.record { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); }
        match snapshot.durable_state {
            DurableReconnectStateV1::Prepared => { if snapshot.current_generation.is_some() || snapshot.current_transport_ref.is_some() { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); } self.phase = ReconnectDurabilityPhaseV1::AwaitFinalRevalidation; Ok(ReconnectProjectionDecisionV1::AwaitFinalRevalidation) }
            DurableReconnectStateV1::Committed => { if snapshot.current_generation != Some(self.record.connection().candidate()) || snapshot.current_transport_ref != Some(self.record.connection().transport_ref()) || current.observed_at > self.record.authorization_deadline()? || !current_authority_matches_record(&self.record, &current)? { return Err(ReconnectDurabilityErrorV1::ReconciliationMismatch); } self.phase = ReconnectDurabilityPhaseV1::Completed; Ok(ReconnectProjectionDecisionV1::InstallController { generation: self.record.connection().candidate(), transport_ref: self.record.connection().transport_ref() }) }
            DurableReconnectStateV1::Terminal => { self.phase = ReconnectDurabilityPhaseV1::Terminal; Ok(ReconnectProjectionDecisionV1::Terminal) }
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

    fn sample_record(attempt_raw: u64, transport_byte: u8) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let game_session_id = GameSessionId::decode(&uuid_v7(10)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let character_id = CharacterId::decode(&uuid_v7(11)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let world_id = WorldId::decode(&uuid_v7(12)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let channel_id = ChannelId::decode(&uuid_v7(13)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let identity = ReconnectIdentityV1::new(game_session_id, ReconnectAttemptRef::new(attempt_raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, "123e4567-e89b-12d3-a456-426614174000", character_id, world_id, RuntimeScopeRefV1::channel(world_id, channel_id))?;
        let predecessor = ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let candidate = ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[transport_byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let connection = ReconnectConnectionFenceV1::new(predecessor, candidate, transport_ref)?;
        let authority = ReconnectAuthorityFenceV1::new(9, ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?;
        let continuity = ReconnectContinuityV1::new(ControlLossEpochRefV1::new(3)?, 120, 115, ProtectionEntitlementV1::unused())?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, vec![PendingCommandReconciliationV1::new(CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, PendingCommandDispositionV1::PendingOriginal), PendingCommandReconciliationV1::new(CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, PendingCommandDispositionV1::TerminalOutcomeRetained)], 41, vec![StateDomainRevisionV1::new(1, 4)?, StateDomainRevisionV1::new(2, 7)?])?;
        let platform = AuthorityEvidenceFenceV1::new("platform-security", "reconnect", "account", "sec:17", "decision:sec:17", 100)?;
        let trust = AuthorityEvidenceFenceV1::new("proof-trust", "reconnect", "recovery-key", "trust:21", "decision:trust:21", 101)?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(1, 1, "rules:1", "content:2", "map:3", "world:4", 12, platform, trust, Some(110))?;
        ReconnectDurabilityRecordV1::new(identity, connection, authority, continuity, ReconnectProofV1::ReauthenticatedRecovery { recovery_grant_nonce: [0x55; 32] }, fnd02, compatibility)
    }

    #[test]
    fn authenticated_transport_ref_v1_is_exact_nonzero_16_bytes() -> Result<(), AdmissionError> {
        let encoded = [0xA5u8; 16];
        let transport_ref = AuthenticatedTransportRefV1::decode(&encoded)?;
        assert_eq!(transport_ref.to_bytes(), encoded);
        assert_eq!(AuthenticatedTransportRefV1::decode(&[0u8; 16]), Err(AdmissionError::InvalidFacts));
        assert_eq!(AuthenticatedTransportRefV1::decode(&[0xA5u8; 15]), Err(AdmissionError::InvalidFacts));
        Ok(())
    }

    #[test]
    fn same_attempt_ref_is_immutable_and_ninth_attempt_fails_before_allocation() -> Result<(), ReconnectDurabilityErrorV1> {
        let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(9)?);
        let first_attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let first_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let changed_ref = AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::New);
        assert_eq!(budget.reserve(first_attempt, first_ref)?, ReconnectAttemptReservationV1::Existing);
        assert_eq!(budget.reserve(first_attempt, changed_ref), Err(ReconnectDurabilityErrorV1::IdempotencyConflict));
        for raw in 2u64..=8 {
            let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            assert_eq!(budget.reserve(ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?, ReconnectAttemptReservationV1::New);
        }
        assert_eq!(budget.reserve(ReconnectAttemptRef::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[9u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?), Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded));
        assert_eq!(budget.distinct_attempts(), 8);
        Ok(())
    }

    #[test]
    fn collision_allows_only_new_attempt_under_capacity_and_never_same_attempt_remint() -> Result<(), ReconnectDurabilityErrorV1> {
        let mut budget = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(1)?);
        let attempt = ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let transport_ref = AuthenticatedTransportRefV1::decode(&[1u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        budget.reserve(attempt, transport_ref)?;
        budget.accept_prepare_completion(attempt, transport_ref, ReconnectPrepareDispositionV1::RejectedTransportRefCollision)?;
        assert!(budget.replacement_allowed_after_collision(attempt));
        assert_eq!(budget.reserve(attempt, AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?), Err(ReconnectDurabilityErrorV1::IdempotencyConflict));
        assert_eq!(budget.reserve(ReconnectAttemptRef::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[2u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?, ReconnectAttemptReservationV1::New);
        let mut exhausted = ReconnectAttemptBudgetV1::new(ControlLossEpochRefV1::new(2)?);
        for raw in 1u64..=8 {
            let byte = u8::try_from(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
            exhausted.reserve(ReconnectAttemptRef::new(raw).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[byte; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?)?;
        }
        exhausted.accept_prepare_completion(ReconnectAttemptRef::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, AuthenticatedTransportRefV1::decode(&[8u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?, ReconnectPrepareDispositionV1::RejectedTransportRefCollision)?;
        assert!(!exhausted.replacement_allowed_after_collision(ReconnectAttemptRef::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?));
        Ok(())
    }

    #[test]
    fn record_preserves_complete_authority_reconciliation_and_security_evidence() -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(3, 3)?;
        assert_eq!(record.version(), 1);
        assert_eq!(record.identity().account_id(), "123e4567-e89b-12d3-a456-426614174000");
        assert!(matches!(record.identity().runtime_scope(), RuntimeScopeRefV1::Channel { .. }));
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
        assert_eq!(record.compatibility().platform_security_evidence().source_observed_at(), 100);
        assert_eq!(record.compatibility().proof_trust_evidence().source_observed_at(), 101);
        Ok(())
    }

    #[test]
    fn prepare_unavailable_retries_same_request_and_ambiguous_requires_reconciliation() -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(4, 4)?;
        let (mut unavailable_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(unavailable_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Unavailable))?, ReconnectPrepareActionV1::RetrySameRequest(request.clone()));
        assert_eq!(unavailable_flow.phase(), ReconnectDurabilityPhaseV1::PendingPrepare);
        let (mut ambiguous_flow, ambiguous_request) = ReconnectDurabilityFlowV1::begin(record);
        assert_eq!(ambiguous_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&ambiguous_request, ReconnectPrepareDispositionV1::Ambiguous))?, ReconnectPrepareActionV1::ReconcileSameAttempt);
        assert_eq!(ambiguous_flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);
        Ok(())
    }

    #[test]
    fn ambiguous_same_attempt_reconciliation_projects_terminal() -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(7, 7)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Ambiguous))?, ReconnectPrepareActionV1::ReconcileSameAttempt);
        let snapshot = ReconnectDurableReconciliationSnapshotV1::terminal(record.clone());
        assert_eq!(snapshot.record, record);
        assert_eq!(snapshot.durable_state, DurableReconnectStateV1::Terminal);
        assert_eq!(snapshot.current_generation, None);
        assert_eq!(snapshot.current_transport_ref, None);
        assert_eq!(flow.accept_reconciliation(snapshot, ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?, ReconnectProjectionDecisionV1::Terminal);
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::Terminal);
        Ok(())
    }

    #[test]
    fn prepared_completion_requires_fresh_complete_revalidation_before_commit() -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(5, 5)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Prepared))?, ReconnectPrepareActionV1::AwaitFinalRevalidation);
        let current = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        let commit = flow.authorize_commit(current, 104)?;
        assert_eq!(commit.authorization().authorization_deadline(), 105);
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);
        let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        stale_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&stale_request, ReconnectPrepareDispositionV1::Prepared))?;
        let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        stale.account_security_generation = stale.account_security_generation.checked_add(1).ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        assert_eq!(stale_flow.authorize_commit(stale, 104), Err(ReconnectDurabilityErrorV1::StaleAuthority));
        Ok(())
    }

    #[test]
    fn ambiguous_commit_installs_controller_only_after_exact_committed_reconciliation() -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(6, 6)?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&request, ReconnectPrepareDispositionV1::Prepared))?;
        let commit_request = flow.authorize_commit(ReconnectCurrentAuthorityV1::from_record(&record, 105)?, 104)?;
        assert_eq!(flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(&commit_request, ReconnectCommitDispositionV1::Ambiguous))?, ReconnectCommitActionV1::ReconcileSameAttempt);
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);
        assert_eq!(flow.accept_reconciliation(ReconnectDurableReconciliationSnapshotV1::committed(record.clone()), ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?, ReconnectProjectionDecisionV1::InstallController { generation: record.connection().candidate(), transport_ref: record.connection().transport_ref() });
        let (mut mismatch_flow, mismatch_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        mismatch_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(&mismatch_request, ReconnectPrepareDispositionV1::Ambiguous))?;
        let mut mismatch = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
        mismatch.current_transport_ref = Some(AuthenticatedTransportRefV1::decode(&[7u8; 16]).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?);
        assert_eq!(mismatch_flow.accept_reconciliation(mismatch, ReconnectCurrentAuthorityV1::from_record(&record, 105)?), Err(ReconnectDurabilityErrorV1::ReconciliationMismatch));
        Ok(())
    }
}

impl ReconnectDurabilityFlowV2 {
    pub fn authorize_commit(
        &mut self,
        current: ReconnectCurrentAuthorityV1,
        now: i64,
    ) -> Result<ReconnectCommitRequestV1, ReconnectDurabilityErrorV1> {
        if self.phase != ReconnectDurabilityPhaseV1::AwaitFinalRevalidation {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        if !current_authority_matches_record(&self.record, &current)?
            || !authenticated_evidence_observed_by(&self.record, now)
        {
            self.phase = ReconnectDurabilityPhaseV1::Terminal;
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        let deadline = self.record.authorization_deadline()?;
        if now > deadline || current.observed_at > deadline {
            self.phase = ReconnectDurabilityPhaseV1::Terminal;
            return Err(ReconnectDurabilityErrorV1::DeadlineExpired);
        }
        let request = ReconnectCommitRequestV1 {
            record: Box::new(self.record.clone()),
            authorization: ReconnectCommitAuthorizationV1 {
                authorization_deadline: deadline,
            },
        };
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
        let expected = ReconnectCommitRequestV1 {
            record: Box::new(self.record.clone()),
            authorization: ReconnectCommitAuthorizationV1 {
                authorization_deadline: self.record.authorization_deadline()?,
            },
        };
        if completion.request != expected {
            return Err(ReconnectDurabilityErrorV1::CompletionMismatch);
        }
        match completion.disposition {
            ReconnectCommitDispositionV1::Unavailable => {
                Ok(ReconnectCommitActionV1::RetrySameRequest(completion.request))
            }
            ReconnectCommitDispositionV1::Committed | ReconnectCommitDispositionV1::Ambiguous => {
                self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired;
                Ok(ReconnectCommitActionV1::ReconcileSameAttempt)
            }
            terminal => {
                self.phase = ReconnectDurabilityPhaseV1::Terminal;
                Ok(ReconnectCommitActionV1::Terminal(terminal))
            }
        }
    }
}

fn current_authority_matches_record(
    record: &ReconnectDurabilityRecordV1,
    current: &ReconnectCurrentAuthorityV1,
) -> Result<bool, ReconnectDurabilityErrorV1> {
    let identity = record.identity();
    let compatibility = record.compatibility();
    Ok(authenticated_evidence_observed_by(record, current.observed_at)
        && current.identity == *identity
        && current.current_account_presence
            == Some(AccountPresenceClaimV1::expected_from_identity(identity)?)
        && current.current_character_world_eligibility
            == Some(CharacterWorldEligibilityClaimV1::expected_from_identity(identity))
        && current.current_candidate
            == Some(ReconnectCandidateBindingV1::expected_binding_from_record(record)?)
        && current.current_runtime_scope == identity.runtime_scope()
        && current.predecessor == record.connection().predecessor()
        && current.authority == record.authority()
        && current.continuity_epoch == record.continuity().control_loss_epoch()
        && current.original_grace_deadline == record.continuity().original_grace_deadline()
        && current.proof == *record.proof()
        && current.fnd02 == *record.fnd02()
        && current.protocol_major == compatibility.protocol_major()
        && current.transport_profile == compatibility.transport_profile()
        && current.ruleset_revision == compatibility.ruleset_revision()
        && current.content_revision == compatibility.content_revision()
        && current.map_revision == compatibility.map_revision()
        && current.world_policy_revision == compatibility.world_policy_revision()
        && current.account_security_generation == compatibility.account_security_generation()
        && current.platform_security_evidence == *compatibility.platform_security_evidence()
        && current.proof_trust_evidence == *compatibility.proof_trust_evidence()
        && current.credential_expiration == compatibility.credential_expiration()
        && current
            .current_candidate
            .is_some_and(|candidate| candidate.is_live_at(current.observed_at))
        && current.session_state == GameSessionState::Reconnectable
        && !current.current_controller_present)
}

fn authenticated_evidence_observed_by(record: &ReconnectDurabilityRecordV1, observed_at: i64) -> bool {
    let compatibility = record.compatibility();
    observed_at >= compatibility.platform_security_evidence().source_observed_at()
        && observed_at >= compatibility.proof_trust_evidence().source_observed_at()
}

#[cfg(test)]
mod durability_reconnect_v2_commit_phase_regression_tests {
    use super::*;

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut out = [0_u8; 16];
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
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(7)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(8)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
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
            vec![],
            41,
            vec![],
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
    fn direct_and_reconciled_v2_prepared_paths_preserve_commit_progression(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let direct_record = sample_record(20, 20)?;
        let mut direct_budget =
            ReconnectAttemptBudgetV1::new(direct_record.continuity().control_loss_epoch());
        direct_budget.reserve(
            direct_record.identity().reconnect_attempt_ref(),
            direct_record.connection().transport_ref(),
        )?;
        let (mut direct_flow, direct_request) =
            ReconnectDurabilityFlowV2::begin(direct_record.clone(), None);
        assert_eq!(
            direct_flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &direct_request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut direct_budget,
            )?,
            ReconnectPrepareActionV2::AwaitFinalRevalidation
        );
        let direct_commit = direct_flow.authorize_commit(
            ReconnectCurrentAuthorityV1::from_record(&direct_record, 105)?,
            104,
        )?;
        assert_eq!(direct_flow.phase(), ReconnectDurabilityPhaseV1::PendingCommit);
        assert_eq!(
            direct_flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                &direct_commit,
                ReconnectCommitDispositionV1::Ambiguous,
            ))?,
            ReconnectCommitActionV1::ReconcileSameAttempt
        );

        let reconciled_record = sample_record(21, 21)?;
        let mut reconciled_budget =
            ReconnectAttemptBudgetV1::new(reconciled_record.continuity().control_loss_epoch());
        reconciled_budget.reserve(
            reconciled_record.identity().reconnect_attempt_ref(),
            reconciled_record.connection().transport_ref(),
        )?;
        let (mut reconciled_flow, reconciled_request) =
            ReconnectDurabilityFlowV2::begin(reconciled_record.clone(), None);
        assert_eq!(
            reconciled_flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &reconciled_request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut reconciled_budget,
            )?,
            ReconnectPrepareActionV2::ReconcileSameAttempt
        );
        assert_eq!(
            reconciled_flow.accept_reconciliation(
                ReconnectDurableReconciliationSnapshotV2::new(
                    reconciled_record.clone(),
                    ReconnectDurableOutcomeV2::Prepared,
                ),
                ReconnectCurrentAuthorityV1::from_record(&reconciled_record, 105)?,
                &mut reconciled_budget,
            )?,
            ReconnectProjectionDecisionV2::AwaitFinalRevalidation
        );
        let reconciled_commit = reconciled_flow.authorize_commit(
            ReconnectCurrentAuthorityV1::from_record(&reconciled_record, 105)?,
            104,
        )?;
        assert_eq!(
            reconciled_flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                &reconciled_commit,
                ReconnectCommitDispositionV1::Committed,
            ))?,
            ReconnectCommitActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            reconciled_flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
        Ok(())
    }

    #[test]
    fn v2_committed_reconciliation_revalidates_complete_current_authority(
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record(31, 31)?;
        let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            },
        );
        let reconcile = |current: ReconnectCurrentAuthorityV1| {
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget.reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )?;
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut budget,
            )?;
            flow.accept_reconciliation(snapshot.clone(), current, &mut budget)
        };

        assert!(matches!(
            reconcile(ReconnectCurrentAuthorityV1::from_record(&record, 105)?)?,
            ReconnectProjectionDecisionV2::InstallController { .. }
        ));
        assert_eq!(
            reconcile(ReconnectCurrentAuthorityV1::from_record(&record, 106)?),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        let mut stale = ReconnectCurrentAuthorityV1::from_record(&record, 105)?;
        stale.session_state = GameSessionState::Terminal;
        assert_eq!(
            reconcile(stale),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        Ok(())
    }
}

/// Historical actor loss-epoch finality. Session terminality alone does not
/// close an otherwise eligible actor epoch; restoration and retirement do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryEpochStateV1 {
    Open,
    Restored,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedRecoveryAttemptDispositionV1 {
    Committed,
    Prepared,
    TransportCollision,
    Terminal,
}

/// One retained budget entry. These are historical facts, not live authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetainedRecoveryAttemptV1 {
    pub attempt: ReconnectAttemptRef,
    pub transport: AuthenticatedTransportRefV1,
    pub disposition: RetainedRecoveryAttemptDispositionV1,
}

/// Complete retained actor-bound budget. There is deliberately no Default/new
/// empty constructor; restart must explicitly establish completeness. A public
/// historical value cannot register the separately sealed current actor source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRecoveryBudgetV1 {
    epoch: ControlLossEpochRefV1,
    state: RecoveryEpochStateV1,
    entries: Vec<RetainedRecoveryAttemptV1>,
}
impl RetainedRecoveryBudgetV1 {
    pub fn restore(
        epoch: ControlLossEpochRefV1,
        state: RecoveryEpochStateV1,
        complete: bool,
        entries: Vec<RetainedRecoveryAttemptV1>,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if !complete || entries.len() > RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1
            || entries.iter().filter(|entry|entry.disposition==RetainedRecoveryAttemptDispositionV1::Committed).count()>1
            || (state==RecoveryEpochStateV1::Open && entries.iter().any(|entry|entry.disposition==RetainedRecoveryAttemptDispositionV1::Committed))
            || entries.iter().enumerate().any(|(index, entry)| entries[..index].iter().any(|prior| prior.attempt == entry.attempt))
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(Self { epoch, state, entries })
    }
    #[must_use]
    pub const fn epoch(&self) -> ControlLossEpochRefV1 { self.epoch }
    #[must_use]
    pub const fn state(&self) -> RecoveryEpochStateV1 { self.state }
    #[must_use]
    pub fn entries(&self) -> &[RetainedRecoveryAttemptV1] { &self.entries }
    pub fn check_candidate(
        &self,
        attempt: ReconnectAttemptRef,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<ReconnectAttemptReservationV1, ReconnectDurabilityErrorV1> {
        if self.state != RecoveryEpochStateV1::Open {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.attempt == attempt) {
            if entry.transport != transport { return Err(ReconnectDurabilityErrorV1::IdempotencyConflict); }
            return match entry.disposition {
                RetainedRecoveryAttemptDispositionV1::Prepared => Ok(ReconnectAttemptReservationV1::Existing),
                RetainedRecoveryAttemptDispositionV1::Committed | RetainedRecoveryAttemptDispositionV1::TransportCollision | RetainedRecoveryAttemptDispositionV1::Terminal => Err(ReconnectDurabilityErrorV1::StaleAuthority),
            };
        }
        if self.entries.len() >= RECONNECT_ATTEMPTS_PER_LOSS_EPOCH_V1 {
            return Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded);
        }
        Ok(ReconnectAttemptReservationV1::New)
    }
}

/// Closed historical timing representation. This public value alone cannot
/// select a live post-grace operation; that requires sealed current sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryTimingV2 {
    SameSession(ReconnectContinuityV1),
    TerminalSessionPostGrace {
        original_grace_deadline: i64,
        attempt_deadline: i64,
    },
}
impl RecoveryTimingV2 {
    pub fn validate_at(self, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        let valid = match self {
            Self::SameSession(continuity) => now >= 0 && now <= continuity.prepared_deadline() && now <= continuity.original_grace_deadline(),
            Self::TerminalSessionPostGrace { original_grace_deadline, attempt_deadline } => original_grace_deadline > 0 && attempt_deadline > original_grace_deadline && now > original_grace_deadline && now <= attempt_deadline,
        };
        if valid { Ok(()) } else { Err(ReconnectDurabilityErrorV1::StaleAuthority) }
    }
}

/// Retained protection history, never inferred from an empty session row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryProtectionUseV1 {
    NotEntitled,
    Unused { entitlement_generation: u64 },
    Activated { entitlement_generation: u64, activated_at: i64, deadline: i64 },
}
/// Source-authored stable-control evidence. This representation chooses no
/// re-arm threshold and cannot start or restart its timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryProtectionRearmV1 {
    NotRearmed { generation: u64, stable_control_started_at: Option<i64>, accepted_deadline: Option<i64> },
    Satisfied { generation: u64, established_at: i64 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryProtectionContinuityV1 {
    pub usage: RecoveryProtectionUseV1,
    pub rearm: RecoveryProtectionRearmV1,
}
impl RecoveryProtectionContinuityV1 {
    fn validate(self, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        let usage_valid = match self.usage {
            RecoveryProtectionUseV1::NotEntitled => true,
            RecoveryProtectionUseV1::Unused { entitlement_generation } => entitlement_generation > 0,
            RecoveryProtectionUseV1::Activated { entitlement_generation, activated_at, deadline } => entitlement_generation > 0 && activated_at >= 0 && activated_at <= now && activated_at.checked_add(4) == Some(deadline),
        };
        let rearm_valid = match self.rearm {
            RecoveryProtectionRearmV1::NotRearmed { generation, stable_control_started_at, accepted_deadline } => generation > 0 && match (stable_control_started_at, accepted_deadline) {
                (None, None) => true,
                (Some(start), Some(deadline)) => start >= 0 && start <= now && deadline > start,
                _ => false,
            },
            RecoveryProtectionRearmV1::Satisfied { generation, established_at } => generation > 0 && established_at >= 0 && established_at <= now,
        };
        if usage_valid && rearm_valid { Ok(()) } else { Err(ReconnectDurabilityErrorV1::InvalidRecord) }
    }
}

/// An inert observation returned only through the sealed Game owning source.
/// Placement identity is an opaque owner-authored placement binding, not a peer
/// coordinate or relocation request. All mutable facts are independently read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceActorObservationV1 {
    pub source_authority: String,
    pub source_revision: u64,
    pub accepted_source_revision: u64,
    pub decision_identity: String,
    pub accepted_decision_identity: String,
    pub source_observed_at: i64,
    pub current: super::fnd04_verifier::RecoveryCurrentEvidence,
    pub predecessor: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub account_presence: Option<AccountPresenceClaimV1>,
    pub present_uncontrolled: bool,
    pub runtime_ready: bool,
    pub reconciliation: Fnd02ReconciliationFenceV1,
    pub placement_identity: [u8; 16],
    pub placement_revision: u64,
    /// Shared accepted Account security source floor across fresh and recovery.
    pub account_security_source_revision: u64,
    pub budget: RetainedRecoveryBudgetV1,
    pub protection: Option<RecoveryProtectionContinuityV1>,
}
impl PostGraceActorObservationV1 {
    pub(super) fn validate_resource_fields(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        if !super::fnd04_verifier::recovery_lifecycle_fields_bounded(&[&self.source_authority,&self.decision_identity,&self.accepted_decision_identity,&self.current.account_id,&self.current.ruleset_revision,&self.current.content_revision,&self.current.map_revision,&self.current.world_policy_revision]) {return Err(ReconnectDurabilityErrorV1::InvalidRecord);}
        Ok(())
    }
    fn validate(&self, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        self.validate_resource_fields()?;
        let invalid = ReconnectDurabilityErrorV1::StaleAuthority;
        let snapshot = self.predecessor;
        let commit = snapshot.commit();
        if !self.present_uncontrolled || !self.runtime_ready || self.placement_revision == 0 || self.placement_identity == [0; 16]
            || self.source_authority.is_empty() || self.source_revision == 0 || self.source_revision != self.accepted_source_revision
            || self.decision_identity.is_empty() || self.decision_identity != self.accepted_decision_identity
            || self.source_observed_at < 0 || self.source_observed_at > now || self.account_security_source_revision == 0
            || snapshot.session_state() != GameSessionState::Terminal || snapshot.current_transport().is_some()
            || !canonical_uuid(&self.current.account_id) || self.current.character_id != commit.character_id() || self.current.world_id != commit.world_id()
            || snapshot.current_character_lease().character_id() != self.current.character_id
            || snapshot.current_character_world_eligibility() != Some(CharacterWorldEligibilityClaimV1::new(self.current.character_id, self.current.world_id))
            || snapshot.current_runtime_scope().world_id() != self.current.world_id
            || self.account_presence.as_ref().is_none_or(|presence| presence.account_id() != self.current.account_id || presence.character_id() != self.current.character_id)
            || snapshot.current_control_loss_epoch() != Some(self.budget.epoch()) || self.budget.state() != RecoveryEpochStateV1::Open
            || snapshot.current_original_grace_deadline().is_none_or(|grace| grace <= 0 || now <= grace)
        { return Err(invalid); }
        validate_current_authority(commit.game_session_id(), snapshot).map_err(|_| invalid)?;
        self.protection.ok_or(invalid)?.validate(now)?;
        Ok(())
    }
}
/// A DTO or historical receipt cannot implement the registration supertrait.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct History;
/// impl PostGraceActorSourceV1 for History {
///     fn resolve_current_actor(&self, _: &str, _: CharacterId, _: i64) -> Result<PostGraceActorObservationV1, ReconnectDurabilityErrorV1> { unreachable!() }
/// }
/// ```
pub trait PostGraceActorSourceV1: super::fnd04_verifier::recovery_source_sealed::Sealed {
    /// Resolve the owning in-memory projection without SQL/network waiting.
    fn resolve_current_actor(&self, account_id: &str, character_id: CharacterId, now: i64) -> Result<PostGraceActorObservationV1, ReconnectDurabilityErrorV1>;
}
pub struct PostGraceActorAuthorityV1<'a> {
    source: Option<&'a dyn PostGraceActorSourceV1>,
}
impl<'a> PostGraceActorAuthorityV1<'a> {
    #[must_use]
    pub const fn unavailable() -> Self { Self { source: None } }
    #[must_use]
    pub const fn from_owning_source(source: &'a dyn PostGraceActorSourceV1) -> Self { Self { source: Some(source) } }
    fn resolve(&self, account_id: &str, character_id: CharacterId, now: i64) -> Result<PostGraceActorObservationV1, ReconnectDurabilityErrorV1> {
        let result = self.source.ok_or(ReconnectDurabilityErrorV1::StaleAuthority)?.resolve_current_actor(account_id, character_id, now)?;
        result.validate(now)?;
        if result.current.account_id != account_id || result.current.character_id != character_id { return Err(ReconnectDurabilityErrorV1::StaleAuthority); }
        Ok(result)
    }
}

/// Original admission operation retained byte-for-byte logically across current
/// source refreshes. This is inert history, not a live source or completion.
/// The later claim/flow layer must bind its exact owner-authored transition too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceRecoveryOperationV1 {
    pub version: u16,
    pub credential: super::fnd04_verifier::RecoveryCredentialAuditV2,
    pub actor: PostGraceActorObservationV1,
    pub candidate: GameSessionId,
    pub candidate_generation: ConnectionGeneration,
    pub attempt: ReconnectAttemptRef,
    pub transport: AuthenticatedTransportRefV1,
    pub timing: RecoveryTimingV2,
    pub prepared_at: i64,
}

/// Private live eligibility. No historical timing, observation, receipt or caller
/// flag can construct this capability. It remains inert until its exact owning
/// transaction; final authorization must independently resolve current sources.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn forge(history: PostGraceActorObservationV1) -> PostGraceRecoveryAuthorizationV1 { history.into() }
/// ```
#[derive(Debug, Clone)]
pub struct PostGraceRecoveryAuthorizationV1 {
    operation: PostGraceRecoveryOperationV1,
    verified: super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV2,
    actor: PostGraceActorObservationV1,
    candidate: GameSessionId,
    attempt: ReconnectAttemptRef,
    transport: AuthenticatedTransportRefV1,
    deadline: i64,
    prepared_at: i64,
}
impl PostGraceRecoveryAuthorizationV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        verified: &super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV2,
        trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>,
        authority: &PostGraceActorAuthorityV1<'_>,
        candidate: GameSessionId,
        attempt: ReconnectAttemptRef,
        transport: AuthenticatedTransportRefV1,
        now: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        let actor = authority.resolve(verified.facts().account_id(), verified.facts().character_id(), now)?;
        let verified = verified.revalidate(now, trust, &actor.current).map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;
        if candidate == actor.predecessor.commit().game_session_id() || actor.account_security_source_revision != verified.security().provenance.source_revision {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        actor.budget.check_candidate(attempt, transport)?;
        let deadline = verified.accepted_deadline();
        RecoveryTimingV2::TerminalSessionPostGrace {
            original_grace_deadline: actor.predecessor.current_original_grace_deadline().ok_or(ReconnectDurabilityErrorV1::StaleAuthority)?,
            attempt_deadline: deadline,
        }.validate_at(now)?;
        let operation = PostGraceRecoveryOperationV1 {
            version: 1,
            credential: verified.audit(),
            actor: actor.clone(),
            candidate,
            candidate_generation: ConnectionFence::fresh_admission().current(),
            attempt,
            transport,
            timing: RecoveryTimingV2::TerminalSessionPostGrace {
                original_grace_deadline: actor.predecessor.current_original_grace_deadline().ok_or(ReconnectDurabilityErrorV1::StaleAuthority)?,
                attempt_deadline: deadline,
            },
            prepared_at: now,
        };
        Ok(Self { operation, verified, actor, candidate, attempt, transport, deadline, prepared_at: now })
    }
    #[must_use]
    pub const fn attempt_deadline(&self) -> i64 { self.deadline }
    #[must_use]
    pub const fn operation(&self) -> &PostGraceRecoveryOperationV1 { &self.operation }
    #[must_use]
    pub const fn candidate_generation(&self) -> ConnectionGeneration { ConnectionFence::fresh_admission().current() }
    #[must_use]
    pub const fn predecessor(&self) -> GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1> { self.actor.predecessor }
    #[must_use]
    pub const fn candidate(&self) -> GameSessionId { self.candidate }
    #[must_use]
    pub const fn attempt(&self) -> ReconnectAttemptRef { self.attempt }
    #[must_use]
    pub const fn transport(&self) -> AuthenticatedTransportRefV1 { self.transport }
    #[must_use]
    pub const fn actor(&self) -> &PostGraceActorObservationV1 { &self.actor }
    #[must_use]
    pub const fn verified(&self) -> &super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV2 { &self.verified }
    pub fn revalidate(
        &self,
        trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>,
        authority: &PostGraceActorAuthorityV1<'_>,
        now: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        if now < self.prepared_at || now > self.deadline { return Err(ReconnectDurabilityErrorV1::StaleAuthority); }
        let next = Self::prepare(&self.verified, trust, authority, self.candidate, self.attempt, self.transport, now)?;
        let before = &self.actor;
        let after = &next.actor;
        validate_post_grace_actor_successor(before,after,self.attempt)?;
        // A stricter current bound may reject; no refresh may extend this attempt.
        if now > next.deadline { return Err(ReconnectDurabilityErrorV1::StaleAuthority); }
        Ok(Self { operation: self.operation.clone(), deadline: self.deadline, prepared_at: self.prepared_at, ..next })
    }
}

impl PostGraceRecoveryOperationV1 {
    /// Historical consistency only. Callers cannot upgrade this result into
    /// live preparation, completion, claims or controller authority.
    pub fn validate_historical(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        let invalid=ReconnectDurabilityErrorV1::InvalidRecord;
        self.credential.validate_historical().map_err(|_| invalid)?;
        self.actor.validate(self.prepared_at)?;
        self.actor.budget.check_candidate(self.attempt,self.transport)?;
        let expected=RecoveryTimingV2::TerminalSessionPostGrace {
            original_grace_deadline:self.actor.predecessor.current_original_grace_deadline().ok_or(invalid)?,
            attempt_deadline:self.credential.accepted_deadline,
        };
        if self.version!=1 || self.timing!=expected || self.prepared_at<self.credential.verified_at
            || self.candidate==self.actor.predecessor.commit().game_session_id() || self.candidate_generation.get()!=1
            || self.credential.account_id!=self.actor.current.account_id || self.credential.character_id!=self.actor.current.character_id || self.credential.world_id!=self.actor.current.world_id
            || self.credential.ruleset_revision!=self.actor.current.ruleset_revision || self.credential.content_revision!=self.actor.current.content_revision
            || self.credential.map_revision!=self.actor.current.map_revision || self.credential.world_policy_revision!=self.actor.current.world_policy_revision
            || self.credential.security.provenance.source_revision!=self.actor.account_security_source_revision { return Err(invalid); }
        expected.validate_at(self.prepared_at)
    }
}

fn validate_post_grace_actor_successor(before: &PostGraceActorObservationV1, after: &PostGraceActorObservationV1, attempt: ReconnectAttemptRef) -> Result<(), ReconnectDurabilityErrorV1> {
    before.validate_resource_fields()?; after.validate_resource_fields()?;
        if before.source_authority != after.source_authority || after.source_revision < before.source_revision || after.source_observed_at < before.source_observed_at
            || (before.source_revision == after.source_revision && before != after)
            || (before.source_revision < after.source_revision && before.decision_identity == after.decision_identity)
            || before.current != after.current || before.predecessor != after.predecessor
            || before.account_presence != after.account_presence || before.placement_identity != after.placement_identity || before.placement_revision != after.placement_revision
            || before.reconciliation != after.reconciliation
            || before.protection != after.protection || before.budget.epoch() != after.budget.epoch()
            || before.budget.entries().iter().any(|entry| entry.attempt == attempt && !after.budget.entries().contains(entry))
            || before.budget.entries().iter().filter(|entry| entry.attempt != attempt).ne(after.budget.entries().iter().filter(|entry| entry.attempt != attempt))
        { return Err(ReconnectDurabilityErrorV1::StaleAuthority); }
    Ok(())
}

/// Exact claim/decision-time observations, separate from the immutable original
/// admission operation. Stored audit is not a live authorization capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceRecoveryDecisionAuditV1 {
    pub credential: super::fnd04_verifier::RecoveryCredentialAuditV2,
    pub actor: PostGraceActorObservationV1,
}
impl PostGraceRecoveryDecisionAuditV1 {
    pub fn validate_for_operation(&self, operation: &PostGraceRecoveryOperationV1, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        operation.validate_historical()?;
        self.credential.validate_successor_of(&operation.credential,now).map_err(|_|ReconnectDurabilityErrorV1::StaleAuthority)?;
        self.actor.validate(now)?;
        validate_post_grace_actor_successor(&operation.actor,&self.actor,operation.attempt)?;
        self.actor.budget.check_candidate(operation.attempt,operation.transport)?;
        if now>operation.credential.accepted_deadline || self.actor.account_security_source_revision!=self.credential.security.provenance.source_revision {return Err(ReconnectDurabilityErrorV1::StaleAuthority);}
        Ok(())
    }
    pub fn validate_successor_of(&self, prior: &Self, operation: &PostGraceRecoveryOperationV1, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        self.validate_for_operation(operation,now)?;
        self.credential.validate_successor_of(&prior.credential,now).map_err(|_|ReconnectDurabilityErrorV1::StaleAuthority)?;
        validate_post_grace_actor_successor(&prior.actor,&self.actor,operation.attempt)
    }
}
impl PostGraceRecoveryAuthorizationV1 {
    #[must_use]
    pub fn decision_audit(&self) -> PostGraceRecoveryDecisionAuditV1 {
        PostGraceRecoveryDecisionAuditV1 {credential:self.verified.audit(),actor:self.actor.clone()}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostGraceSubmissionV1 { Accepted, Unavailable }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostGraceRequestKindV1 { Prepare, Commit, Reconcile }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostGraceFlowPhaseV1 { Ready, PendingPrepare, Prepared, PendingCommit, ReconciliationRequired, PendingReconciliation, AwaitingAdoption, Adopted, Rejected }

/// Private request construction prevents historical DTOs from selecting a live
/// PREPARE/COMMIT. Reconciliation carries only the original immutable operation.
#[derive(Debug, Clone)]
pub struct PostGraceDurabilityRequestV1 {
    kind: PostGraceRequestKindV1,
    operation: super::admission_authority_publication::PostGraceClaimEvidenceV1,
    authorization: Option<Box<PostGraceRecoveryAuthorizationV1>>,
    claims: Option<Box<super::admission_authority_publication::PostGraceClaimTransitionV1>>,
}
impl PostGraceDurabilityRequestV1 {
    #[must_use]
    pub const fn kind(&self) -> PostGraceRequestKindV1 { self.kind }
    #[must_use]
    pub const fn operation(&self) -> &super::admission_authority_publication::PostGraceClaimEvidenceV1 { &self.operation }
    /// Pure bounded decision over source contexts backed by one independently
    /// locked canonical session/actor/claim/shared-floor snapshot. The adapter
    /// acquires every serialization protection before sampling database time.
    /// PREPARE reserves only; only Commit may apply the exact claim successors.
    pub fn validate_locked(&self, trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>, actor: &PostGraceActorAuthorityV1<'_>, rows: &[Option<super::admission_authority_publication::AdmissionAuthorityPublicationChangeV1>], now: i64) -> Result<PostGraceRecoveryDecisionAuditV1, ReconnectDurabilityErrorV1> {
        if self.kind==PostGraceRequestKindV1::Reconcile {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let authorization=self.authorization.as_ref().ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?;
        let current=authorization.revalidate(trust,actor,now)?;
        self.claims.as_ref().ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?.validate_current(&current,rows,now).map_err(|_|ReconnectDurabilityErrorV1::StaleAuthority)?;
        let decision=current.decision_audit();
        decision.validate_successor_of(&self.operation.authorization,&self.operation.operation,now)?;
        Ok(decision)
    }
}
/// Implementations enqueue into the accepted bounded executor and return. No
/// SQL, connection-pool wait, network wait or detached work on the FND-03 writer.
pub trait PostGraceDurabilityPortV1 {
    fn submit(&mut self, request: &PostGraceDurabilityRequestV1) -> PostGraceSubmissionV1;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostGraceTerminalReasonV1 {
    TransportCollision,
    AttemptCapacityExceeded,
    StaleAuthority,
    DeadlineExpired,
    EpochClosed,
    InvalidOperation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostGraceDurableOutcomeV1 {
    Prepared,
    Committed { decided_at: i64, decision: Box<PostGraceRecoveryDecisionAuditV1> },
    Rejected { reason: PostGraceTerminalReasonV1 },
    Ambiguous,
}
/// Raw durable report. It grants nothing without a registered completion source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceDurableCompletionV1 {
    pub operation: super::admission_authority_publication::PostGraceClaimEvidenceV1,
    pub phase: PostGraceFlowPhaseV1,
    pub outcome: PostGraceDurableOutcomeV1,
}
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct Peer;
/// impl PostGraceCompletionSourceV1 for Peer {
/// fn take_completion(&mut self, _: &oteryn_game_server::foundation::admission_authority_publication::PostGraceClaimEvidenceV1, _: PostGraceFlowPhaseV1) -> Result<Option<PostGraceDurableCompletionV1>, ReconnectDurabilityErrorV1> { Ok(None) }
/// }
/// ```
pub trait PostGraceCompletionSourceV1: super::fnd04_verifier::recovery_source_sealed::Sealed {
    fn take_completion(&mut self, operation: &super::admission_authority_publication::PostGraceClaimEvidenceV1, phase: PostGraceFlowPhaseV1) -> Result<Option<PostGraceDurableCompletionV1>, ReconnectDurabilityErrorV1>;
}
/// Historical committed proof, created only after a sealed exact-operation
/// completion. It is not a controller or a source registration capability.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn install(history: PostGraceCommitReceiptV1) -> PostGraceControllerBindingV1 { history.into() }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceCommitReceiptV1 {
    operation: super::admission_authority_publication::PostGraceClaimEvidenceV1,
    decided_at: i64,
    decision: Box<PostGraceRecoveryDecisionAuditV1>,
}
impl PostGraceCommitReceiptV1 {
    #[must_use]
    pub const fn operation(&self) -> &super::admission_authority_publication::PostGraceClaimEvidenceV1 { &self.operation }
    #[must_use]
    pub const fn decided_at(&self) -> i64 { self.decided_at }
    #[must_use]
    pub fn decision(&self) -> &PostGraceRecoveryDecisionAuditV1 { &self.decision }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostGraceControllerBindingV1 {
    session: GameSessionId,
    generation: ConnectionGeneration,
    transport: AuthenticatedTransportRefV1,
}
impl PostGraceControllerBindingV1 {
    #[must_use]
    pub const fn session(self) -> GameSessionId {self.session}
    #[must_use]
    pub const fn generation(self) -> ConnectionGeneration {self.generation}
    #[must_use]
    pub const fn transport(self) -> AuthenticatedTransportRefV1 {self.transport}
}
#[derive(Debug, Clone)]
pub struct PostGraceDurabilityFlowV1 {
    operation: super::admission_authority_publication::PostGraceClaimEvidenceV1,
    authorization: Option<Box<PostGraceRecoveryAuthorizationV1>>,
    claims: Option<Box<super::admission_authority_publication::PostGraceClaimTransitionV1>>,
    phase: PostGraceFlowPhaseV1,
    receipt: Option<PostGraceCommitReceiptV1>,
    terminal_reason: Option<PostGraceTerminalReasonV1>,
    controller: Option<PostGraceControllerBindingV1>,
}
impl PostGraceDurabilityFlowV1 {
    pub fn begin(authorization: PostGraceRecoveryAuthorizationV1, owner: &dyn super::admission_authority_publication::PostGraceClaimOwningSourceV1, now: i64) -> Result<Self, ReconnectDurabilityErrorV1> {
        let claims=super::admission_authority_publication::PostGraceClaimTransitionV1::prepare(owner,&authorization,now).map_err(|_|ReconnectDurabilityErrorV1::StaleAuthority)?;
        Ok(Self {operation:claims.evidence().clone(),authorization:Some(Box::new(authorization)),claims:Some(Box::new(claims)),phase:PostGraceFlowPhaseV1::Ready,receipt:None,terminal_reason:None,controller:None})
    }
    /// Raw history can request only exact reconciliation, never PREPARE/COMMIT.
    pub fn restore_history(operation: super::admission_authority_publication::PostGraceClaimEvidenceV1) -> Result<Self, ReconnectDurabilityErrorV1> {
        operation.validate_historical(operation.transition.prepared_at).map_err(|_|ReconnectDurabilityErrorV1::InvalidRecord)?;
        Ok(Self {operation,authorization:None,claims:None,phase:PostGraceFlowPhaseV1::ReconciliationRequired,receipt:None,terminal_reason:None,controller:None})
    }
    #[must_use]
    pub const fn operation(&self) -> &super::admission_authority_publication::PostGraceClaimEvidenceV1 {&self.operation}
    #[must_use]
    pub const fn phase(&self) -> PostGraceFlowPhaseV1 {self.phase}
    #[must_use]
    pub const fn receipt(&self) -> Option<&PostGraceCommitReceiptV1> {self.receipt.as_ref()}
    #[must_use]
    pub const fn controller(&self) -> Option<PostGraceControllerBindingV1> {self.controller}
    #[must_use]
    pub const fn terminal_reason(&self) -> Option<PostGraceTerminalReasonV1> {self.terminal_reason}
    pub fn submit_prepare(&mut self, port: &mut dyn PostGraceDurabilityPortV1) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase!=PostGraceFlowPhaseV1::Ready {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let request=PostGraceDurabilityRequestV1 {kind:PostGraceRequestKindV1::Prepare,operation:self.operation.clone(),authorization:self.authorization.clone(),claims:self.claims.clone()};
        if port.submit(&request)==PostGraceSubmissionV1::Accepted {self.phase=PostGraceFlowPhaseV1::PendingPrepare;}
        Ok(())
    }
    pub fn submit_commit(&mut self, port: &mut dyn PostGraceDurabilityPortV1, trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>, actor: &PostGraceActorAuthorityV1<'_>, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase!=PostGraceFlowPhaseV1::Prepared {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let current=self.authorization.as_ref().ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?.revalidate(trust,actor,now)?;
        current.decision_audit().validate_successor_of(&self.operation.authorization,&self.operation.operation,now)?;
        let request=PostGraceDurabilityRequestV1 {kind:PostGraceRequestKindV1::Commit,operation:self.operation.clone(),authorization:Some(Box::new(current.clone())),claims:self.claims.clone()};
        if port.submit(&request)==PostGraceSubmissionV1::Accepted {self.authorization=Some(Box::new(current));self.phase=PostGraceFlowPhaseV1::PendingCommit;}
        Ok(())
    }
    pub fn reconcile(&mut self, port: &mut dyn PostGraceDurabilityPortV1) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase!=PostGraceFlowPhaseV1::ReconciliationRequired {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let request=PostGraceDurabilityRequestV1 {kind:PostGraceRequestKindV1::Reconcile,operation:self.operation.clone(),authorization:None,claims:None};
        if port.submit(&request)==PostGraceSubmissionV1::Accepted {self.phase=PostGraceFlowPhaseV1::PendingReconciliation;}
        Ok(())
    }
    /// No public accept-completion DTO route exists. Missing/ambiguous outcomes
    /// retain the original identity and require bounded exact reconciliation.
    pub fn poll(&mut self, source: &mut dyn PostGraceCompletionSourceV1) -> Result<bool, ReconnectDurabilityErrorV1> {
        if !matches!(self.phase,PostGraceFlowPhaseV1::PendingPrepare|PostGraceFlowPhaseV1::PendingCommit|PostGraceFlowPhaseV1::PendingReconciliation) {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let Some(completion)=source.take_completion(&self.operation,self.phase)? else {return Ok(false);};
        completion.operation.validate_historical(completion.operation.transition.prepared_at).map_err(|_|ReconnectDurabilityErrorV1::CompletionMismatch)?;
        if completion.operation!=self.operation || completion.phase!=self.phase {return Err(ReconnectDurabilityErrorV1::CompletionMismatch);}
        match completion.outcome {
            PostGraceDurableOutcomeV1::Prepared => {
                if self.phase==PostGraceFlowPhaseV1::PendingCommit {return Err(ReconnectDurabilityErrorV1::CompletionMismatch);}
                self.phase=PostGraceFlowPhaseV1::Prepared;
            }
            PostGraceDurableOutcomeV1::Committed {decided_at,decision} => {
                self.operation.validate_historical(decided_at).map_err(|_|ReconnectDurabilityErrorV1::CompletionMismatch)?;
                decision.validate_successor_of(&self.operation.authorization,&self.operation.operation,decided_at)?;
                self.receipt=Some(PostGraceCommitReceiptV1 {operation:self.operation.clone(),decided_at,decision});
                self.phase=PostGraceFlowPhaseV1::AwaitingAdoption;
            }
            PostGraceDurableOutcomeV1::Rejected {reason} => {self.phase=PostGraceFlowPhaseV1::Rejected;self.terminal_reason=Some(reason);self.controller=None;}
            PostGraceDurableOutcomeV1::Ambiguous => {self.phase=PostGraceFlowPhaseV1::ReconciliationRequired;self.controller=None;}
        }
        Ok(true)
    }
}

/// Independently resolved current owning-source facts. Historical receipt bytes
/// cannot implement the sealed source or create a controller projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostGraceAdoptionCurrentV1 {
    pub actor: PostGraceActorObservationV1,
    pub session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub actor_present: bool,
    pub controller: Option<(GameSessionId, ConnectionGeneration, AuthenticatedTransportRefV1)>,
    pub live_transport: Option<AuthenticatedTransportRefV1>,
    pub security: super::fnd04_verifier::RecoveryAccountSecurityObservationV2,
    pub signing: super::fnd04_verifier::RecoverySigningTrustObservationV2,
    pub claims: Vec<super::admission_authority_publication::AdmissionAuthorityPublicationChangeV1>,
}
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct PeerHistory;
/// impl PostGraceAdoptionSourceV1 for PeerHistory {
/// fn current_adoption(&self, _: &oteryn_game_server::foundation::admission_authority_publication::PostGraceClaimEvidenceV1, _: i64) -> Result<PostGraceAdoptionCurrentV1, ReconnectDurabilityErrorV1> { unreachable!() }
/// }
/// ```
pub trait PostGraceAdoptionSourceV1: super::fnd04_verifier::recovery_source_sealed::Sealed {
    fn current_adoption(&self, operation: &super::admission_authority_publication::PostGraceClaimEvidenceV1, now: i64) -> Result<PostGraceAdoptionCurrentV1, ReconnectDurabilityErrorV1>;
}
impl PostGraceDurabilityFlowV1 {
    /// Direct and reconciled durable success share this current adoption fence.
    /// Failure always clears the local controller projection.
    pub fn adopt(&mut self, source: &dyn PostGraceAdoptionSourceV1, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        self.controller = None;
        if !matches!(self.phase, PostGraceFlowPhaseV1::AwaitingAdoption | PostGraceFlowPhaseV1::Adopted) {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        self.phase = PostGraceFlowPhaseV1::AwaitingAdoption;
        let receipt = self.receipt.as_ref().ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?;
        let current = source.current_adoption(&self.operation, now)?;
        validate_post_grace_adoption(receipt, &current, now)?;
        let operation = &self.operation.operation;
        self.controller = Some(PostGraceControllerBindingV1 {session: operation.candidate, generation: operation.candidate_generation, transport: operation.transport});
        self.phase = PostGraceFlowPhaseV1::Adopted;
        Ok(())
    }
}
fn validate_post_grace_adoption(receipt: &PostGraceCommitReceiptV1, current: &PostGraceAdoptionCurrentV1, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
    let stale = ReconnectDurabilityErrorV1::StaleAuthority;
    current.actor.validate_resource_fields()?;
    receipt.decision.actor.validate_resource_fields()?;
    super::admission_authority_publication::validate_post_grace_claim_resource_fields(&current.claims).map_err(|_|stale)?;
    let operation = &receipt.operation.operation;
    let prior = &receipt.decision.actor;
    let actor = &current.actor;
    let session = current.session;
    super::fnd04_verifier::validate_recovery_adoption_sources(&receipt.decision.credential, &current.signing, &current.security, now).map_err(|_|stale)?;
    if now < receipt.decided_at || !current.actor_present || actor.present_uncontrolled || !actor.runtime_ready
        || actor.source_authority != prior.source_authority || actor.source_revision <= prior.source_revision
        || actor.source_revision != actor.accepted_source_revision || actor.decision_identity.is_empty()
        || actor.decision_identity != actor.accepted_decision_identity || actor.decision_identity == prior.decision_identity
        || actor.source_observed_at < receipt.decided_at || actor.source_observed_at > now
        || actor.current != prior.current || actor.predecessor != prior.predecessor
        || actor.account_presence != prior.account_presence || actor.placement_identity != prior.placement_identity
        || actor.placement_revision != prior.placement_revision || actor.reconciliation != prior.reconciliation
        || actor.account_security_source_revision != current.security.provenance.source_revision
        || actor.budget.epoch != prior.budget.epoch || actor.budget.state != RecoveryEpochStateV1::Restored
        || current.controller != Some((operation.candidate, operation.candidate_generation, operation.transport))
        || current.live_transport != Some(operation.transport)
        || session.session_state != GameSessionState::Active || session.commit.game_session_id() != operation.candidate
        || session.current_connection_generation != operation.candidate_generation || session.current_transport != Some(operation.transport)
        || session.commit.connection_generation() != operation.candidate_generation || session.commit.initial_transport() != operation.transport
        || session.commit.character_lease_generation() != prior.predecessor.current_character_lease.generation()
        || session.commit.scope_ownership_generation() != prior.predecessor.current_scope_generation.get()
        || session.commit.character_id() != prior.current.character_id || session.commit.world_id() != prior.current.world_id
        || session.commit.channel_id() != prior.predecessor.commit.channel_id()
        || session.current_character_lease != prior.predecessor.current_character_lease
        || session.current_character_world_eligibility != prior.predecessor.current_character_world_eligibility
        || session.current_runtime_scope != prior.predecessor.current_runtime_scope
        || session.current_scope_generation != prior.predecessor.current_scope_generation
        || session.current_control_loss_epoch != Some(prior.budget.epoch())
        || session.current_original_grace_deadline != prior.predecessor.current_original_grace_deadline
        || current.claims != receipt.operation.transition.successors
    { return Err(stale); }
    let committed = actor.budget.entries.iter().filter(|entry| entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed).collect::<Vec<_>>();
    if committed.len() != 1 || committed[0].attempt != operation.attempt || committed[0].transport != operation.transport
        || prior.budget.entries.iter().any(|old| !actor.budget.entries.iter().any(|next| old.attempt == next.attempt && old.transport == next.transport &&
            (if old.attempt == operation.attempt {next.disposition == RetainedRecoveryAttemptDispositionV1::Committed}
             else {next.disposition == old.disposition || (old.disposition == RetainedRecoveryAttemptDispositionV1::Prepared && next.disposition == RetainedRecoveryAttemptDispositionV1::Terminal)})))
        || actor.budget.entries.iter().any(|next| next.attempt != operation.attempt && !prior.budget.entries.iter().any(|old| old.attempt == next.attempt))
    { return Err(stale); }
    let protection = actor.protection.as_ref().ok_or(stale)?;
    let old_protection = prior.protection.as_ref().ok_or(stale)?;
    protection.validate(now)?;
    let expected_usage = match old_protection.usage {
        RecoveryProtectionUseV1::Unused {entitlement_generation} => RecoveryProtectionUseV1::Activated {entitlement_generation, activated_at: receipt.decided_at, deadline: receipt.decided_at.checked_add(4).ok_or(stale)?},
        usage => usage,
    };
    if protection.usage != expected_usage || protection.rearm != old_protection.rearm {return Err(stale);}
    super::admission_authority_publication::validate_post_grace_adoption_claims(&operation.credential.account_id, session, &current.claims).map_err(|_|stale)?;
    Ok(())
}

impl PostGraceRecoveryAuthorizationV1 {
    /// History supplies identity only. A freshly verified credential and current
    /// sealed sources must independently authorize the exact unchanged operation.
    pub fn reauthorize_history(operation: PostGraceRecoveryOperationV1, verified: super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV2, trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>, actor: &PostGraceActorAuthorityV1<'_>, now: i64) -> Result<Self, ReconnectDurabilityErrorV1> {
        operation.validate_historical()?;
        let mut current = Self::prepare(&verified, trust, actor, operation.candidate, operation.attempt, operation.transport, now)?;
        current.decision_audit().validate_for_operation(&operation, now)?;
        current.deadline = operation.credential.accepted_deadline;
        current.prepared_at = operation.prepared_at;
        current.operation = operation;
        Ok(current)
    }
}
impl PostGraceDurabilityFlowV1 {
    /// Only a sealed reconciliation report of PREPARED permits this resumption.
    /// Raw history alone remains unable to reserve or commit an attempt.
    pub fn resume_prepared(&mut self, authorization: PostGraceRecoveryAuthorizationV1, trust: &super::fnd04_verifier::RecoveryDurabilityTrustContextV2<'_>, actor: &PostGraceActorAuthorityV1<'_>, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase != PostGraceFlowPhaseV1::Prepared {return Err(ReconnectDurabilityErrorV1::InvalidPhase);}
        let current = authorization.revalidate(trust, actor, now)?;
        let claims = super::admission_authority_publication::PostGraceClaimTransitionV1::resume_prepared(self.operation.clone(), &current, now).map_err(|_|ReconnectDurabilityErrorV1::StaleAuthority)?;
        self.authorization = Some(Box::new(current));
        self.claims = Some(Box::new(claims));
        Ok(())
    }
}

#[cfg(test)]
#[path = "control_loss_durability_tests.rs"]
mod control_loss_durability_tests;

/// Classification from the owning runtime, never a caller supplied loss flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLossCauseV1 {
    AuthoritativeUnexpectedLoss,
    HealthyController,
    SocketClosedOnly,
    ProcessRestartOnly,
    GracefulLogout,
    HealthyMigration,
    Suspected,
}
/// Complete prior continuity. Fresh origin is asserted by the sealed owner,
/// not inferred from a missing database row. Resumed history remains retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlLossHistoryV1 {
    FreshOrigin,
    Resumed {
        budget: RetainedRecoveryBudgetV1,
        original_grace_deadline: i64,
        protection: RecoveryProtectionContinuityV1,
    },
}
/// Inert source observation; constructing it does not grant live authority.
/// Source identity uses the existing runtime scope plus the snapshot ownership
/// generation; decision identity is the existing owner-issued loss epoch, bound
/// to its complete origin/grace evidence. No new identity protocol or lossy
/// string conversion is introduced. All fields are fixed-width except the
/// existing canonical UUID account claim and at-most-eight retained attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLossObservationV1 {
    pub source_authority: RuntimeScopeRefV1,
    pub source_revision: u64,
    pub accepted_source_revision: u64,
    pub decision_identity: ControlLossEpochRefV1,
    pub accepted_decision_identity: ControlLossEpochRefV1,
    pub observed_at: i64,
    pub session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub account_presence: AccountPresenceClaimV1,
    pub placement_identity: [u8; 16],
    pub placement_revision: u64,
    pub actor_present: bool,
    pub runtime_ready: bool,
    pub cause: ControlLossCauseV1,
    pub loss_epoch: ControlLossEpochRefV1,
    pub loss_origin: i64,
    pub original_grace_deadline: i64,
    pub history: ControlLossHistoryV1,
    pub protection: RecoveryProtectionContinuityV1,
}
impl ControlLossObservationV1 {
    fn validate(&self, now: i64) -> Result<(), ReconnectDurabilityErrorV1> {
        let stale = ReconnectDurabilityErrorV1::StaleAuthority;
        if self.source_authority != self.session.current_runtime_scope()
            || self.source_revision == 0
            || self.source_revision != self.accepted_source_revision
            || self.decision_identity != self.loss_epoch
            || self.decision_identity != self.accepted_decision_identity
            || self.observed_at < 0
            || self.observed_at > now
            || self.loss_origin < 0
            || self.loss_origin > self.observed_at
            || self.original_grace_deadline <= self.loss_origin
            || !self.actor_present
            || !self.runtime_ready
            || self.placement_identity == [0; 16]
            || self.placement_revision == 0
            || self.cause != ControlLossCauseV1::AuthoritativeUnexpectedLoss
            || self.session.session_state() != GameSessionState::Active
            || self.session.current_transport().is_none()
            || self.account_presence.character_id() != self.session.commit().character_id()
        {
            return Err(stale);
        }
        validate_current_authority(self.session.commit().game_session_id(), self.session)
            .map_err(|_| stale)?;
        self.protection.validate(self.loss_origin)?;
        match &self.history {
            ControlLossHistoryV1::FreshOrigin => {
                if self.session.current_control_loss_epoch().is_some()
                    || self.session.current_original_grace_deadline().is_some()
                    || self.session.current_connection_generation()
                        != self.session.commit().connection_generation()
                    || self.session.current_transport()
                        != Some(self.session.commit().initial_transport())
                    || matches!(
                        self.protection.usage,
                        RecoveryProtectionUseV1::Activated { .. }
                    )
                {
                    return Err(stale);
                }
            }
            ControlLossHistoryV1::Resumed {
                budget,
                original_grace_deadline,
                protection,
            } => {
                if budget.state() != RecoveryEpochStateV1::Restored
                    || budget.epoch().get() >= self.loss_epoch.get()
                    || self.session.current_control_loss_epoch() != Some(budget.epoch())
                    || self.session.current_original_grace_deadline()
                        != Some(*original_grace_deadline)
                    || *original_grace_deadline <= 0
                    || !budget.entries().iter().any(|entry| {
                        entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed
                            && Some(entry.transport) == self.session.current_transport()
                    })
                    || self.protection != *protection
                {
                    return Err(stale);
                }
                protection.validate(self.loss_origin)?;
            }
        }
        Ok(())
    }
}
/// A registered owning runtime independently resolves the current session,
/// controller, loss decision and complete retained continuity. It must not
/// manufacture observations from a request/receipt. Resolution is bounded and
/// does not wait on SQL or network. Actual producer registration is separate.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct Socket;
/// impl ControlLossSourceV1 for Socket {
/// fn resolve_loss(&self, _: GameSessionId, _: i64) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> { unreachable!() }
/// }
/// ```
pub trait ControlLossSourceV1: super::fnd04_verifier::recovery_source_sealed::Sealed {
    fn resolve_loss(
        &self,
        session: GameSessionId,
        now: i64,
    ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1>;
}
/// Immutable original operation. This is history, not an authorization token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLossOperationV1 {
    pub version: u16,
    pub observation: ControlLossObservationV1,
    pub authorized_at: i64,
}
impl ControlLossOperationV1 {
    fn validate_historical(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.version != 1 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        self.observation.validate(self.authorized_at)
    }
}
/// Only Foundation constructs this capability from an independent sealed owner.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn forge(operation: ControlLossOperationV1) -> ControlLossAuthorizationV1 {
/// ControlLossAuthorizationV1 { operation }
/// }
/// ```
#[derive(Debug)]
pub struct ControlLossAuthorizationV1 {
    operation: ControlLossOperationV1,
}
impl ControlLossAuthorizationV1 {
    pub fn authorize(
        source: &dyn ControlLossSourceV1,
        session: GameSessionId,
        now: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        let observation = source.resolve_loss(session, now)?;
        observation.validate(now)?;
        if observation.session.commit().game_session_id() != session {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        Ok(Self {
            operation: ControlLossOperationV1 {
                version: 1,
                observation,
                authorized_at: now,
            },
        })
    }
    #[must_use]
    pub const fn operation(&self) -> &ControlLossOperationV1 {
        &self.operation
    }
    /// Pure final predicate for the later adapter's locked atomic boundary.
    /// Caller must apply only the returned exact effect under the same fences.
    /// Historical retry uses reconciliation; this method never replays a write.
    pub fn validate_final(
        &self,
        source: &dyn ControlLossSourceV1,
        now: i64,
    ) -> Result<ControlLossEffectV1, ReconnectDurabilityErrorV1> {
        let original = &self.operation.observation;
        let mut current = source.resolve_loss(original.session.commit().game_session_id(), now)?;
        current.validate(now)?;
        if now < self.operation.authorized_at
            || current.source_revision < original.source_revision
            || current.observed_at < original.observed_at
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        // A newer observation may confirm this exact immutable owning decision;
        // changes to authority, event, claims or any continuity remain forbidden.
        current.source_revision = original.source_revision;
        current.accepted_source_revision = original.accepted_source_revision;
        current.observed_at = original.observed_at;
        if current != *original {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        let mut successor = original.session;
        successor.session_state = GameSessionState::Reconnectable;
        successor.current_transport = None;
        successor.current_control_loss_epoch = Some(original.loss_epoch);
        successor.current_original_grace_deadline = Some(original.original_grace_deadline);
        Ok(ControlLossEffectV1 {
            operation: self.operation.clone(),
            successor,
        })
    }
}
/// Exact bounded write projection, privately constructed after final validation.
/// It contains no claim acquisition/release or actor/protection mutation.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn forge(operation: ControlLossOperationV1, successor: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>) -> ControlLossEffectV1 {
/// ControlLossEffectV1 {operation, successor}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLossEffectV1 {
    operation: ControlLossOperationV1,
    successor: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
}
impl ControlLossEffectV1 {
    #[must_use]
    pub const fn operation(&self) -> &ControlLossOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn predecessor(&self) -> GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1> {
        self.operation.observation.session
    }
    #[must_use]
    pub const fn successor(&self) -> GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1> {
        self.successor
    }
}

/// A live adapter request can only be taken once from an authorized flow.
/// The adapter must call validate_final with its independently current owning
/// source under the same durable fences as the exact atomic loss mutation.
#[derive(Debug)]
pub struct ControlLossRequestV1 {
    authorization: ControlLossAuthorizationV1,
}
impl ControlLossRequestV1 {
    #[must_use]
    pub const fn operation(&self) -> &ControlLossOperationV1 {
        self.authorization.operation()
    }
    pub fn validate_final(
        &self,
        source: &dyn ControlLossSourceV1,
        now: i64,
    ) -> Result<ControlLossEffectV1, ReconnectDurabilityErrorV1> {
        self.authorization.validate_final(source, now)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLossOutcomeV1 {
    Committed { decided_at: i64 },
    Rejected,
    Ambiguous,
}
/// Historical report; only a registered completion source can deliver it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLossCompletionV1 {
    pub operation: ControlLossOperationV1,
    pub outcome: ControlLossOutcomeV1,
}
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct History;
/// impl ControlLossCompletionSourceV1 for History {
/// fn take_loss_completion(&mut self, _: &ControlLossOperationV1) -> Result<Option<ControlLossCompletionV1>,ReconnectDurabilityErrorV1> { Ok(None) }
/// }
/// ```
pub trait ControlLossCompletionSourceV1:
    super::fnd04_verifier::recovery_source_sealed::Sealed
{
    fn take_loss_completion(
        &mut self,
        operation: &ControlLossOperationV1,
    ) -> Result<Option<ControlLossCompletionV1>, ReconnectDurabilityErrorV1>;
}
/// Inert original disposition. No receipt-to-live or receipt-to-effect conversion.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn replay(receipt: ControlLossReceiptV1) -> ControlLossRequestV1 {receipt.into()}
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLossReceiptV1 {
    operation: ControlLossOperationV1,
    decided_at: i64,
}
impl ControlLossReceiptV1 {
    #[must_use]
    pub const fn operation(&self) -> &ControlLossOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn decided_at(&self) -> i64 {
        self.decided_at
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLossPhaseV1 {
    Ready,
    Pending,
    ReconciliationRequired,
    Completed,
    Rejected,
}
#[derive(Debug)]
pub struct ControlLossFlowV1 {
    operation: ControlLossOperationV1,
    authorization: Option<ControlLossAuthorizationV1>,
    phase: ControlLossPhaseV1,
    receipt: Option<ControlLossReceiptV1>,
}
impl ControlLossFlowV1 {
    #[must_use]
    pub fn begin(authorization: ControlLossAuthorizationV1) -> Self {
        Self {
            operation: authorization.operation().clone(),
            authorization: Some(authorization),
            phase: ControlLossPhaseV1::Ready,
            receipt: None,
        }
    }
    /// Restored public history permits only read/reconciliation. Even a valid
    /// historical committed operation cannot yield another live write request.
    pub fn restore(operation: ControlLossOperationV1) -> Result<Self, ReconnectDurabilityErrorV1> {
        operation.validate_historical()?;
        Ok(Self {
            operation,
            authorization: None,
            phase: ControlLossPhaseV1::ReconciliationRequired,
            receipt: None,
        })
    }
    #[must_use]
    pub const fn operation(&self) -> &ControlLossOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn phase(&self) -> ControlLossPhaseV1 {
        self.phase
    }
    #[must_use]
    pub const fn receipt(&self) -> Option<&ControlLossReceiptV1> {
        self.receipt.as_ref()
    }
    pub fn take_request(&mut self) -> Result<ControlLossRequestV1, ReconnectDurabilityErrorV1> {
        if self.phase != ControlLossPhaseV1::Ready {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let authorization = self
            .authorization
            .take()
            .ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?;
        self.phase = ControlLossPhaseV1::Pending;
        Ok(ControlLossRequestV1 { authorization })
    }
    /// Completion classifies persistence only. It does not mutate a current
    /// session/controller projection and does not require stale history to match
    /// a superseding live controller. Exact repeated reports preserve disposition.
    pub fn accept_completion(
        &mut self,
        source: &mut dyn ControlLossCompletionSourceV1,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase == ControlLossPhaseV1::Ready {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let Some(completion) = source.take_loss_completion(&self.operation)? else {
            return Ok(());
        };
        if completion.operation != self.operation {
            return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
        }
        match completion.outcome {
            ControlLossOutcomeV1::Committed { decided_at } => {
                if decided_at < self.operation.authorized_at
                    || self.phase == ControlLossPhaseV1::Rejected
                {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                let receipt = ControlLossReceiptV1 {
                    operation: self.operation.clone(),
                    decided_at,
                };
                if self.receipt.as_ref().is_some_and(|prior| *prior != receipt) {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                self.receipt = Some(receipt);
                self.phase = ControlLossPhaseV1::Completed;
            }
            ControlLossOutcomeV1::Rejected => {
                if self.phase == ControlLossPhaseV1::Completed {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                self.phase = ControlLossPhaseV1::Rejected;
            }
            ControlLossOutcomeV1::Ambiguous => {
                if !matches!(
                    self.phase,
                    ControlLossPhaseV1::Completed | ControlLossPhaseV1::Rejected
                ) {
                    self.phase = ControlLossPhaseV1::ReconciliationRequired;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "control_loss_reconnect_bridge_tests.rs"]
mod control_loss_reconnect_bridge_tests;

/// Complete reconnect is an additive durability format, not a wire version.
/// Legacy reconnect records cannot represent an absent protection entitlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteReconnectModeV1 {
    SameSession,
    EarlyTerminalReplacement,
}

/// V1 exposes no source revisions. These are exact values requested by the
/// existing verifier and returned by its authenticated authority, not V2 audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteV1TrustBindingV1 {
    pub signing_key_id: String,
    pub signing_public_key: [u8; 32],
    pub minimum_generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteRecoveryCredentialV1 {
    pub grant_nonce: [u8; 32],
    pub account_security_generation: u64,
    pub protocol_major: u64,
    pub transport_profile: u64,
    pub revisions: [String; 4],
    pub expires_at: i64,
    pub v1_trust: Option<CompleteV1TrustBindingV1>,
    pub v2: Option<super::fnd04_verifier::RecoveryCredentialAuditV2>,
}
impl CompleteRecoveryCredentialV1 {
    fn from_verified(facts: &super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV1) -> Self {
        Self {
            grant_nonce: facts.grant_nonce(),
            account_security_generation: facts.account_security_generation(),
            protocol_major: facts.protocol_major(),
            transport_profile: facts.transport_profile(),
            revisions: [
                facts.ruleset_revision().into(),
                facts.content_revision().into(),
                facts.map_revision().into(),
                facts.world_policy_revision().into(),
            ],
            expires_at: facts.credential_expiration(),
            v1_trust: None,
            v2: None,
        }
    }
    fn validate(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.grant_nonce == [0; 32]
            || self.account_security_generation == 0
            || self.protocol_major != 1
            || self.transport_profile != 1
            || self.expires_at <= 0
            || !super::fnd04_verifier::recovery_lifecycle_fields_bounded(
                &self
                    .revisions
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        match (&self.v1_trust, &self.v2) {
            (Some(binding), None)
                if binding.minimum_generation > 0
                    && binding.minimum_generation <= self.account_security_generation
                    && !binding.signing_key_id.is_empty()
                    && super::fnd04_verifier::recovery_lifecycle_fields_bounded(&[
                        &binding.signing_key_id
                    ]) => {}
            (None, Some(_)) => {}
            _ => return Err(ReconnectDurabilityErrorV1::InvalidRecord),
        }
        if let Some(audit) = &self.v2 {
            audit
                .validate_historical()
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        }
        Ok(())
    }
}

/// Inert metadata from the registered proof owner after verifying the actual
/// bearer on this authenticated transport. Generation DTOs are not proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteFastReconnectBindingV1 {
    pub session: GameSessionId,
    pub predecessor: ConnectionGeneration,
    pub attempt: ReconnectAttemptRef,
    pub transport: AuthenticatedTransportRefV1,
    pub proof_generation: u64,
    pub replacement_proof_generation: u64,
    pub verified_at: i64,
    pub compatibility: ReconnectCompatibilityEvidenceV1,
}
impl CompleteFastReconnectBindingV1 {
    fn validate(
        &self,
        current: &CompleteReconnectSnapshotV1,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let compatibility = &self.compatibility;
        if self.session != identity.game_session_id()
            || self.session != current.session.commit().game_session_id()
            || self.predecessor != current.session.current_connection_generation()
            || self.attempt != identity.reconnect_attempt_ref()
            || self.transport != current.candidate.transport_ref()
            || self.proof_generation != current.proof_transition.predecessor_generation
            || self.replacement_proof_generation != current.proof_transition.successor_generation
            || self.proof_generation == 0
            || self.replacement_proof_generation <= self.proof_generation
            || self.verified_at != now
            || compatibility.credential_expiration().is_some()
            || compatibility.protocol_major() != 1
            || compatibility.transport_profile() != 1
            || compatibility.ruleset_revision() != current.recovery.ruleset_revision
            || compatibility.content_revision() != current.recovery.content_revision
            || compatibility.map_revision() != current.recovery.map_revision
            || compatibility.world_policy_revision() != current.recovery.world_policy_revision
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        for evidence in [
            compatibility.platform_security_evidence(),
            compatibility.proof_trust_evidence(),
        ] {
            if evidence.source_observed_at() > now
                || evidence
                    .source_observed_at()
                    .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                    .is_none_or(|deadline| {
                        now > deadline || current.candidate.prepared_deadline() > deadline
                    })
            {
                return Err(ReconnectDurabilityErrorV1::StaleAuthority);
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteReconnectCredentialV1 {
    Recovery(Box<CompleteRecoveryCredentialV1>),
    Fast(Box<CompleteFastReconnectBindingV1>),
}
impl CompleteReconnectCredentialV1 {
    #[must_use]
    pub fn recovery(&self) -> Option<&CompleteRecoveryCredentialV1> {
        match self {
            Self::Recovery(value) => Some(value),
            Self::Fast(_) => None,
        }
    }
}

/// Owner-reserved inactive successor proof. This contains no secret and grants
/// no authority until the matching COMMIT atomically fences the predecessor.
/// Proof generations are independent of connection and protection generations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectProofTransitionV1 {
    pub owner: RuntimeScopeRefV1,
    pub revision: u64,
    pub accepted_revision: u64,
    pub observed_at: i64,
    pub predecessor_session: GameSessionId,
    pub predecessor_generation: u64,
    pub successor_session: GameSessionId,
    pub successor_generation: u64,
    pub candidate: ReconnectCandidateBindingV1,
}
impl CompleteReconnectProofTransitionV1 {
    fn validate(
        &self,
        current: &CompleteReconnectSnapshotV1,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.owner != current.source_authority
            || self.revision == 0
            || self.accepted_revision != self.revision
            || self.observed_at > now
            || self.observed_at < current.loss_decided_at
            || self
                .observed_at
                .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                .is_none_or(|deadline| now > deadline)
            || self.predecessor_session != current.session.commit().game_session_id()
            || self.predecessor_generation == 0
            || self.successor_session != identity.game_session_id()
            || self.successor_generation == 0
            || self.candidate != current.candidate
            || (self.successor_session == self.predecessor_session
                && self.successor_generation <= self.predecessor_generation)
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        Ok(())
    }
}
/// Independently current proof-owner state after the committed activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectProofCurrentV1 {
    pub owner: RuntimeScopeRefV1,
    pub revision: u64,
    pub accepted_revision: u64,
    pub observed_at: i64,
    pub session: GameSessionId,
    pub connection: ConnectionGeneration,
    pub transport: AuthenticatedTransportRefV1,
    pub proof_generation: u64,
}

/// Independently loaded complete actor/session/claim facts. The original loss
/// is durable history; only the sealed source can attest its current continuity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectSnapshotV1 {
    pub loss: ControlLossOperationV1,
    pub loss_decided_at: i64,
    pub source_authority: RuntimeScopeRefV1,
    pub source_revision: u64,
    pub accepted_source_revision: u64,
    pub observed_at: i64,
    pub session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub account_presence: AccountPresenceClaimV1,
    pub actor_present: bool,
    pub runtime_ready: bool,
    pub placement_identity: [u8; 16],
    pub placement_revision: u64,
    pub protection: RecoveryProtectionContinuityV1,
    pub budget: RetainedRecoveryBudgetV1,
    pub candidate: ReconnectCandidateBindingV1,
    pub proof_transition: CompleteReconnectProofTransitionV1,
    pub fnd02: Fnd02ReconciliationFenceV1,
    pub recovery: super::fnd04_verifier::RecoveryCurrentEvidence,
    pub claims: Vec<super::admission_authority_publication::AdmissionAuthorityPublicationChangeV1>,
}
impl CompleteReconnectSnapshotV1 {
    pub(super) fn validate_resources(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        if !super::fnd04_verifier::recovery_lifecycle_fields_bounded(&[
            &self.recovery.account_id,
            &self.recovery.ruleset_revision,
            &self.recovery.content_revision,
            &self.recovery.map_revision,
            &self.recovery.world_policy_revision,
        ]) {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        super::admission_authority_publication::validate_post_grace_claim_resource_fields(
            &self.claims,
        )
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }
    fn validate(
        &self,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        self.validate_resources()?;
        self.loss.validate_historical()?;
        let loss = &self.loss.observation;
        let session = self.session;
        let stale = ReconnectDurabilityErrorV1::StaleAuthority;
        if self.loss_decided_at < self.loss.authorized_at
            || self.loss_decided_at > now
            || now < self.observed_at
            || self.observed_at < self.loss_decided_at
            || self
                .observed_at
                .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                .is_none_or(|deadline| now > deadline)
            || self.source_revision == 0
            || self.source_revision != self.accepted_source_revision
            || self.source_authority != session.current_runtime_scope()
            || !self.actor_present
            || !self.runtime_ready
            || self.placement_identity == [0; 16]
            || self.placement_revision == 0
            || session.current_transport().is_some()
            || !matches!(
                session.session_state(),
                GameSessionState::Reconnectable | GameSessionState::Terminal
            )
            || session.commit() != loss.session.commit()
            || session.current_connection_generation()
                != loss.session.current_connection_generation()
            || self.account_presence != loss.account_presence
            || session.current_control_loss_epoch() != Some(loss.loss_epoch)
            || session.current_original_grace_deadline() != Some(loss.original_grace_deadline)
            || self.budget.epoch() != loss.loss_epoch
            || self.budget.state() != RecoveryEpochStateV1::Open
            || self.protection != loss.protection
            || identity.account_id() != self.account_presence.account_id()
            || identity.character_id() != self.account_presence.character_id()
            || identity.character_id() != session.commit().character_id()
            || identity.world_id() != session.commit().world_id()
            || identity.runtime_scope() != session.current_runtime_scope()
            || identity.account_id() != self.recovery.account_id
            || identity.character_id() != self.recovery.character_id
            || identity.world_id() != self.recovery.world_id
            || self.candidate.game_session_id() != identity.game_session_id()
            || self.candidate.reconnect_attempt_ref() != identity.reconnect_attempt_ref()
            || session.current_connection_generation().get().checked_add(1)
                != Some(self.candidate.connection_generation().get())
            || !self.candidate.is_live_at(now)
            || self.candidate.prepared_deadline() > loss.original_grace_deadline
            || now > loss.original_grace_deadline
        {
            return Err(stale);
        }
        validate_current_authority(session.commit().game_session_id(), session)
            .map_err(|_| stale)?;
        self.protection.validate(now)?;
        self.proof_transition.validate(self, identity, now)?;
        self.budget.check_candidate(
            identity.reconnect_attempt_ref(),
            self.candidate.transport_ref(),
        )?;
        super::admission_authority_publication::validate_complete_reconnect_claims(
            identity.account_id(),
            session,
            &self.claims,
            now,
        )
        .map_err(|_| stale)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectOperationV1 {
    pub version: u16,
    pub mode: CompleteReconnectModeV1,
    pub identity: ReconnectIdentityV1,
    pub original: CompleteReconnectSnapshotV1,
    pub credential: CompleteReconnectCredentialV1,
    pub prepared_at: i64,
}
impl CompleteReconnectOperationV1 {
    pub fn validate_historical(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.version != 1 {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        self.original.validate(&self.identity, self.prepared_at)?;
        if self
            .original
            .budget
            .entries()
            .iter()
            .any(|entry| entry.disposition == RetainedRecoveryAttemptDispositionV1::Prepared)
        {
            return Err(ReconnectDurabilityErrorV1::ConcurrentPrepared);
        }
        match &self.credential {
            CompleteReconnectCredentialV1::Fast(binding) => {
                binding.validate(&self.original, &self.identity, self.prepared_at)?
            }
            CompleteReconnectCredentialV1::Recovery(credential) => {
                credential.validate()?;
                let current = &self.original.recovery;
                if credential.revisions
                    != [
                        current.ruleset_revision.clone(),
                        current.content_revision.clone(),
                        current.map_revision.clone(),
                        current.world_policy_revision.clone(),
                    ]
                {
                    return Err(ReconnectDurabilityErrorV1::InvalidRecord);
                }
                if let Some(audit) = &credential.v2
                    && (audit.account_id != self.identity.account_id()
                        || audit.character_id != self.identity.character_id()
                        || audit.world_id != self.identity.world_id()
                        || audit.grant_nonce != credential.grant_nonce
                        || audit.account_security_generation
                            != credential.account_security_generation
                        || audit.protocol_major != credential.protocol_major
                        || audit.transport_profile != credential.transport_profile
                        || audit.expires_at != credential.expires_at
                        || audit.verified_at != self.prepared_at
                        || self.original.candidate.prepared_deadline() > audit.accepted_deadline
                        || [
                            &audit.ruleset_revision,
                            &audit.content_revision,
                            &audit.map_revision,
                            &audit.world_policy_revision,
                        ] != credential.revisions.each_ref())
                {
                    return Err(ReconnectDurabilityErrorV1::InvalidRecord);
                }
                if self.original.candidate.prepared_deadline() > credential.expires_at {
                    return Err(ReconnectDurabilityErrorV1::InvalidRecord);
                }
            }
        }
        let security_generation = match &self.credential {
            CompleteReconnectCredentialV1::Recovery(value) => value.account_security_generation,
            CompleteReconnectCredentialV1::Fast(value) => {
                value.compatibility.account_security_generation()
            }
        };
        let super::admission_authority_publication::AdmissionAuthorityGuardStateV1::Account {
            security,
            ..
        } = &self.original.claims[0].state
        else {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        };
        if security_generation < security.minimum_generation {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        let same =
            self.identity.game_session_id() == self.original.session.commit().game_session_id();
        if (self.mode == CompleteReconnectModeV1::SameSession
            && (!same || self.original.session.session_state() != GameSessionState::Reconnectable))
            || (self.mode == CompleteReconnectModeV1::EarlyTerminalReplacement
                && (same
                    || self.original.session.session_state() != GameSessionState::Terminal
                    || self
                        .credential
                        .recovery()
                        .is_none_or(|credential| credential.v2.is_none())))
        {
            return Err(ReconnectDurabilityErrorV1::InvalidRecord);
        }
        Ok(())
    }
    fn prepared_budget(&self) -> Result<RetainedRecoveryBudgetV1, ReconnectDurabilityErrorV1> {
        let mut budget = self.original.budget.clone();
        if budget.check_candidate(
            self.identity.reconnect_attempt_ref(),
            self.original.candidate.transport_ref(),
        )? == ReconnectAttemptReservationV1::New
        {
            budget.entries.push(RetainedRecoveryAttemptV1 {
                attempt: self.identity.reconnect_attempt_ref(),
                transport: self.original.candidate.transport_ref(),
                disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
            });
        }
        Ok(budget)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectCurrentV1 {
    pub snapshot: CompleteReconnectSnapshotV1,
    /// Independently loaded canonical prepared operation, never caller supplied.
    pub prepared: Option<Box<CompleteReconnectOperationV1>>,
}
/// Registered runtime/adapter source. Each call resolves fresh current facts;
/// trust methods provide authenticated source contexts, not cached verifier facts.
/// No SQL/network waits are permitted on the owning writer.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// struct Receipt;
/// impl CompleteReconnectSourceV1 for Receipt {}
/// ```
pub trait CompleteReconnectSourceV1: super::fnd04_verifier::recovery_source_sealed::Sealed {
    fn resolve_reconnect(
        &self,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1>;
    /// Must verify actual bearer possession, predecessor fencing and transport
    /// binding independently on every call; raw stored metadata is insufficient.
    fn verify_fast_reconnect(
        &self,
        _identity: &ReconnectIdentityV1,
        _now: i64,
    ) -> Result<CompleteFastReconnectBindingV1, ReconnectDurabilityErrorV1> {
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    }
    fn recovery_v1_authority(&self) -> Option<&dyn super::fnd04_verifier::Fnd04EvidenceAuthority> {
        None
    }
    fn recovery_v2_source(
        &self,
    ) -> Option<&dyn super::fnd04_verifier::RecoveryDurabilityEvidenceSourceV2> {
        None
    }
}
/// Live credential material is never part of the durable operation/receipt.
pub enum CompleteReconnectProofV1 {
    Fast,
    V1Token(String),
    V2(Box<super::fnd04_verifier::VerifiedRecoveryDurabilityFactsV2>),
}
impl std::fmt::Debug for CompleteReconnectProofV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Fast => "Fast(owner verification required)",
            Self::V1Token(_) => "V1Token(REDACTED)",
            Self::V2(_) => "V2(verified)",
        })
    }
}
struct CompleteV1VerifierAuthority<'a> {
    inner: &'a dyn super::fnd04_verifier::Fnd04EvidenceAuthority,
    signing: std::cell::RefCell<Option<(String, [u8; 32])>>,
    minimum: std::cell::Cell<Option<u64>>,
}
impl super::fnd04_verifier::Fnd04EvidenceAuthority for CompleteV1VerifierAuthority<'_> {
    fn signing_key(
        &self,
        scope: super::fnd04_verifier::Fnd04EvidenceScope,
        key_id: &str,
        now: i64,
    ) -> Result<[u8; 32], super::fnd04_verifier::Fnd04EvidenceError> {
        if !super::fnd04_verifier::recovery_lifecycle_fields_bounded(&[key_id]) {
            return Err(super::fnd04_verifier::Fnd04EvidenceError::UnavailableOrStale);
        }
        let key = self.inner.signing_key(scope, key_id, now)?;
        *self.signing.borrow_mut() = Some((key_id.to_owned(), key));
        Ok(key)
    }
    fn account_minimum_generation(
        &self,
        scope: super::fnd04_verifier::Fnd04EvidenceScope,
        account: &str,
        now: i64,
    ) -> Result<u64, super::fnd04_verifier::Fnd04EvidenceError> {
        let minimum = self.inner.account_minimum_generation(scope, account, now)?;
        self.minimum.set(Some(minimum));
        Ok(minimum)
    }
}
impl CompleteReconnectProofV1 {
    fn verify(
        &self,
        source: &dyn CompleteReconnectSourceV1,
        current: &CompleteReconnectSnapshotV1,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<CompleteReconnectCredentialV1, ReconnectDurabilityErrorV1> {
        use super::fnd04_verifier::{
            RecoveryDurabilityTrustContextV2, RecoveryTrustContext,
            verify_recovery_grant_durability_v1,
        };
        let stale = ReconnectDurabilityErrorV1::StaleAuthority;
        let (facts, audit, v1_trust) = match self {
            Self::Fast => {
                let binding = source.verify_fast_reconnect(identity, now)?;
                binding.validate(current, identity, now)?;
                let minimum = source
                    .recovery_v1_authority()
                    .ok_or(stale)?
                    .account_minimum_generation(
                        super::fnd04_verifier::Fnd04EvidenceScope::ExistingActorRecovery,
                        identity.account_id(),
                        now,
                    )
                    .map_err(|_| stale)?;
                if minimum == 0 || minimum > binding.compatibility.account_security_generation() {
                    return Err(stale);
                }
                return Ok(CompleteReconnectCredentialV1::Fast(Box::new(binding)));
            }
            Self::V1Token(token) => {
                // Existing verifier authenticates signing trust and account security
                // at this boundary. An old VerifiedV1 value cannot substitute.
                let authority = source.recovery_v1_authority().ok_or(stale)?;
                let capture = CompleteV1VerifierAuthority {
                    inner: authority,
                    signing: std::cell::RefCell::new(None),
                    minimum: std::cell::Cell::new(None),
                };
                let facts = verify_recovery_grant_durability_v1(
                    token,
                    now,
                    &RecoveryTrustContext::new(&capture),
                    &current.recovery,
                )
                .map_err(|_| stale)?;
                let (signing_key_id, signing_public_key) =
                    capture.signing.into_inner().ok_or(stale)?;
                let binding = CompleteV1TrustBindingV1 {
                    signing_key_id,
                    signing_public_key,
                    minimum_generation: capture.minimum.get().ok_or(stale)?,
                };
                (facts, None, Some(binding))
            }
            Self::V2(original) => {
                let trust = RecoveryDurabilityTrustContextV2::from_owning_source(
                    source.recovery_v2_source().ok_or(stale)?,
                );
                let next = original
                    .revalidate(now, &trust, &current.recovery)
                    .map_err(|_| stale)?;
                (next.facts().clone(), Some(next.audit()), None)
            }
        };
        if facts.account_id() != identity.account_id()
            || facts.character_id() != identity.character_id()
            || facts.world_id() != identity.world_id()
        {
            return Err(stale);
        }
        let mut result = CompleteRecoveryCredentialV1::from_verified(&facts);
        result.v2 = audit;
        result.v1_trust = v1_trust;
        result.validate()?;
        Ok(CompleteReconnectCredentialV1::Recovery(Box::new(result)))
    }
}

/// Private live authorization; deserialized history has no conversion into it.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn forge(history: CompleteReconnectOperationV1) -> CompleteReconnectAuthorizationV1 { history.into() }
/// ```
#[derive(Debug)]
pub struct CompleteReconnectAuthorizationV1 {
    operation: CompleteReconnectOperationV1,
    proof: CompleteReconnectProofV1,
}
impl CompleteReconnectAuthorizationV1 {
    pub fn authorize(
        source: &dyn CompleteReconnectSourceV1,
        identity: ReconnectIdentityV1,
        proof: CompleteReconnectProofV1,
        now: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        let current = source.resolve_reconnect(&identity, now)?;
        current.snapshot.validate(&identity, now)?;
        if current.prepared.is_some() {
            return Err(ReconnectDurabilityErrorV1::ConcurrentPrepared);
        }
        let credential = proof.verify(source, &current.snapshot, &identity, now)?;
        let mode = if current.snapshot.session.session_state() == GameSessionState::Terminal {
            CompleteReconnectModeV1::EarlyTerminalReplacement
        } else {
            CompleteReconnectModeV1::SameSession
        };
        let operation = CompleteReconnectOperationV1 {
            version: 1,
            mode,
            identity,
            original: current.snapshot,
            credential,
            prepared_at: now,
        };
        operation.validate_historical()?;
        Ok(Self { operation, proof })
    }
    #[must_use]
    pub const fn operation(&self) -> &CompleteReconnectOperationV1 {
        &self.operation
    }
    pub fn reauthorize_history(
        operation: CompleteReconnectOperationV1,
        proof: CompleteReconnectProofV1,
        source: &dyn CompleteReconnectSourceV1,
        now: i64,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        operation.validate_historical()?;
        let result = Self { operation, proof };
        result.validate_current(source, true, now)?;
        Ok(result)
    }
    fn validate_current(
        &self,
        source: &dyn CompleteReconnectSourceV1,
        prepared: bool,
        now: i64,
    ) -> Result<
        (CompleteReconnectSnapshotV1, CompleteReconnectCredentialV1),
        ReconnectDurabilityErrorV1,
    > {
        let operation = &self.operation;
        operation.validate_historical()?;
        let current = source.resolve_reconnect(&operation.identity, now)?;
        current.snapshot.validate(&operation.identity, now)?;
        if now < operation.prepared_at {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        if let Some(stored) = &current.prepared {
            stored.validate_historical()?;
        }
        if prepared {
            if current.prepared.as_deref() != Some(operation)
                || current.snapshot.budget != operation.prepared_budget()?
            {
                return Err(ReconnectDurabilityErrorV1::StaleAuthority);
            }
        } else if current.prepared.is_some() {
            return Err(ReconnectDurabilityErrorV1::ConcurrentPrepared);
        }
        let fresh = self
            .proof
            .verify(source, &current.snapshot, &operation.identity, now)?;
        let mut expected = operation.credential.clone();
        let mut actual = fresh.clone();
        match (&mut expected, &mut actual) {
            (
                CompleteReconnectCredentialV1::Recovery(expected),
                CompleteReconnectCredentialV1::Recovery(actual),
            ) => {
                if let (Some(original), Some(next)) = (&expected.v2, &actual.v2) {
                    next.validate_successor_of(original, now)
                        .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;
                    // Revalidation preserves the original credential and source high-water;
                    // compare immutable signed fields without replacing original audit.
                    if original.credential_attempt_ref != next.credential_attempt_ref
                        || original.grant_nonce != next.grant_nonce
                        || original.issued_at != next.issued_at
                        || original.not_before != next.not_before
                        || original.expires_at != next.expires_at
                    {
                        return Err(ReconnectDurabilityErrorV1::StaleAuthority);
                    }
                    expected.v2 = None;
                    actual.v2 = None;
                }
                if let (Some(prior), Some(next)) = (&expected.v1_trust, &actual.v1_trust) {
                    if prior.signing_key_id != next.signing_key_id
                        || prior.signing_public_key != next.signing_public_key
                        || next.minimum_generation < prior.minimum_generation
                    {
                        return Err(ReconnectDurabilityErrorV1::StaleAuthority);
                    }
                    expected.v1_trust = actual.v1_trust.clone();
                }
            }
            (
                CompleteReconnectCredentialV1::Fast(expected),
                CompleteReconnectCredentialV1::Fast(actual),
            ) => {
                if actual.verified_at < expected.verified_at {
                    return Err(ReconnectDurabilityErrorV1::StaleAuthority);
                }
                actual.verified_at = expected.verified_at;
            }
            _ => return Err(ReconnectDurabilityErrorV1::StaleAuthority),
        }
        if actual != expected {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        let mut normalized = current.snapshot.clone();
        if normalized.source_revision < operation.original.source_revision
            || normalized.observed_at < operation.original.observed_at
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        normalized.source_revision = operation.original.source_revision;
        normalized.accepted_source_revision = operation.original.accepted_source_revision;
        normalized.observed_at = operation.original.observed_at;
        if prepared {
            normalized.budget = operation.original.budget.clone();
        }
        if normalized != operation.original {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        Ok((current.snapshot, fresh))
    }
}

fn complete_reconnect_protection(
    mut protection: RecoveryProtectionContinuityV1,
    now: i64,
) -> Result<RecoveryProtectionContinuityV1, ReconnectDurabilityErrorV1> {
    protection.validate(now)?;
    if let RecoveryProtectionUseV1::Unused {
        entitlement_generation,
    } = protection.usage
    {
        protection.usage = RecoveryProtectionUseV1::Activated {
            entitlement_generation,
            activated_at: now,
            deadline: now
                .checked_add(4)
                .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?,
        };
    }
    Ok(protection)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectDurabilityOperationV1 {
    pub recovery: CompleteReconnectOperationV1,
    pub replacement:
        Option<super::admission_authority_publication::CompleteReconnectClaimEvidenceV1>,
}
impl CompleteReconnectDurabilityOperationV1 {
    pub fn validate_historical(&self) -> Result<(), ReconnectDurabilityErrorV1> {
        self.recovery.validate_historical()?;
        match (&self.replacement, self.recovery.mode) {
            (None, CompleteReconnectModeV1::SameSession) => Ok(()),
            (Some(claims), CompleteReconnectModeV1::EarlyTerminalReplacement) => {
                claims
                    .validate_historical(self.recovery.prepared_at)
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
                if claims.operation != self.recovery {
                    return Err(ReconnectDurabilityErrorV1::InvalidRecord);
                }
                Ok(())
            }
            _ => Err(ReconnectDurabilityErrorV1::InvalidRecord),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteReconnectRequestKindV1 {
    Prepare,
    Commit,
    Reconcile,
}
#[derive(Debug)]
pub struct CompleteReconnectRequestV1 {
    kind: CompleteReconnectRequestKindV1,
    operation: CompleteReconnectDurabilityOperationV1,
    authorization: Option<CompleteReconnectAuthorizationV1>,
    claims: Option<super::admission_authority_publication::CompleteReconnectClaimTransitionV1>,
}
impl CompleteReconnectRequestV1 {
    #[must_use]
    pub const fn kind(&self) -> CompleteReconnectRequestKindV1 {
        self.kind
    }
    #[must_use]
    pub const fn operation(&self) -> &CompleteReconnectDurabilityOperationV1 {
        &self.operation
    }
    /// Source reads must be backed by the adapter's same independently locked
    /// rows/owner/security floors and trusted decision time as the atomic write.
    pub fn validate_locked(
        &self,
        source: &dyn CompleteReconnectSourceV1,
        now: i64,
    ) -> Result<CompleteReconnectEffectV1, ReconnectDurabilityErrorV1> {
        if self.kind == CompleteReconnectRequestKindV1::Reconcile {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let authorization = self
            .authorization
            .as_ref()
            .ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?;
        let commit = self.kind == CompleteReconnectRequestKindV1::Commit;
        let (current, credential) = authorization.validate_current(source, commit, now)?;
        if let Some(claims) = &self.claims {
            claims
                .validate_current(&self.operation.recovery, &current, &credential, now)
                .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;
        }
        complete_reconnect_effect(&self.operation, commit, now)
    }
}
/// Exact inert write projection. PREPARE cannot activate a controller/protection.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn forge(history: CompleteReconnectDurabilityOperationV1) -> CompleteReconnectEffectV1 { history.into() }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectEffectV1 {
    operation: CompleteReconnectDurabilityOperationV1,
    kind: CompleteReconnectRequestKindV1,
    decided_at: i64,
    session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    budget: RetainedRecoveryBudgetV1,
    protection: RecoveryProtectionContinuityV1,
    claims: Vec<super::admission_authority_publication::AdmissionAuthorityPublicationChangeV1>,
}
impl CompleteReconnectEffectV1 {
    /// Applies to fast and reauthenticated recovery alike. The actual proof owner
    /// activates/delivers the reserved successor only with this exact COMMIT.
    #[must_use]
    pub fn proof_transition(&self) -> Option<&CompleteReconnectProofTransitionV1> {
        (self.kind == CompleteReconnectRequestKindV1::Commit)
            .then_some(&self.operation.recovery.original.proof_transition)
    }

    /// Conditional proof-owner rotation metadata; no bearer is issued by this
    /// DTO. The matching commit must atomically fence old proof before delivery.
    #[must_use]
    pub fn fast_proof_rotation(&self) -> Option<(u64, u64)> {
        if self.kind != CompleteReconnectRequestKindV1::Commit {
            return None;
        }
        match &self.operation.recovery.credential {
            CompleteReconnectCredentialV1::Fast(binding) => Some((
                binding.proof_generation,
                binding.replacement_proof_generation,
            )),
            CompleteReconnectCredentialV1::Recovery(_) => None,
        }
    }

    #[must_use]
    pub const fn operation(&self) -> &CompleteReconnectDurabilityOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn kind(&self) -> CompleteReconnectRequestKindV1 {
        self.kind
    }
    #[must_use]
    pub const fn decided_at(&self) -> i64 {
        self.decided_at
    }
    #[must_use]
    pub const fn session(&self) -> GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1> {
        self.session
    }
    #[must_use]
    pub const fn budget(&self) -> &RetainedRecoveryBudgetV1 {
        &self.budget
    }
    #[must_use]
    pub const fn protection(&self) -> RecoveryProtectionContinuityV1 {
        self.protection
    }
    #[must_use]
    pub fn claims(
        &self,
    ) -> &[super::admission_authority_publication::AdmissionAuthorityPublicationChangeV1] {
        &self.claims
    }
}
fn complete_reconnect_effect(
    operation: &CompleteReconnectDurabilityOperationV1,
    commit: bool,
    now: i64,
) -> Result<CompleteReconnectEffectV1, ReconnectDurabilityErrorV1> {
    operation.validate_historical()?;
    let recovery = &operation.recovery;
    if now < recovery.prepared_at || now > recovery.original.candidate.prepared_deadline() {
        return Err(ReconnectDurabilityErrorV1::DeadlineExpired);
    }
    let mut budget = recovery.prepared_budget()?;
    let mut session = recovery.original.session;
    let mut protection = recovery.original.protection;
    let mut claims = recovery.original.claims.clone();
    if commit {
        budget.state = RecoveryEpochStateV1::Restored;
        let winner = budget
            .entries
            .iter_mut()
            .find(|entry| entry.attempt == recovery.identity.reconnect_attempt_ref())
            .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?;
        winner.disposition = RetainedRecoveryAttemptDispositionV1::Committed;
        protection = complete_reconnect_protection(protection, now)?;
        session.session_state = GameSessionState::Active;
        session.current_connection_generation = recovery.original.candidate.connection_generation();
        session.current_transport = Some(recovery.original.candidate.transport_ref());
        if recovery.mode == CompleteReconnectModeV1::EarlyTerminalReplacement {
            session.commit.game_session_id = recovery.identity.game_session_id();
            session.commit.connection_generation = session.current_connection_generation;
            session.commit.initial_transport = recovery.original.candidate.transport_ref();
            session.commit.scope_ownership_generation = session.current_scope_generation.get();
            claims = operation
                .replacement
                .as_ref()
                .ok_or(ReconnectDurabilityErrorV1::InvalidRecord)?
                .transition
                .successors
                .clone();
        }
    }
    Ok(CompleteReconnectEffectV1 {
        operation: operation.clone(),
        kind: if commit {
            CompleteReconnectRequestKindV1::Commit
        } else {
            CompleteReconnectRequestKindV1::Prepare
        },
        decided_at: now,
        session,
        budget,
        protection,
        claims,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteReconnectOutcomeV1 {
    Prepared { decided_at: i64 },
    Committed { decided_at: i64 },
    Rejected,
    Ambiguous,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectCompletionV1 {
    pub operation: CompleteReconnectDurabilityOperationV1,
    pub outcome: CompleteReconnectOutcomeV1,
}
pub trait CompleteReconnectCompletionSourceV1:
    super::fnd04_verifier::recovery_source_sealed::Sealed
{
    fn take_complete_reconnect_completion(
        &mut self,
        operation: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<Option<CompleteReconnectCompletionV1>, ReconnectDurabilityErrorV1>;
}
/// Historical receipt exposes original disposition, never another write effect.
/// ```compile_fail
/// use oteryn_game_server::foundation::*;
/// fn replay(receipt: CompleteReconnectReceiptV1) -> CompleteReconnectRequestV1 { receipt.into() }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteReconnectReceiptV1 {
    operation: CompleteReconnectDurabilityOperationV1,
    decided_at: i64,
}
impl CompleteReconnectReceiptV1 {
    #[must_use]
    pub const fn operation(&self) -> &CompleteReconnectDurabilityOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn decided_at(&self) -> i64 {
        self.decided_at
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteReconnectPhaseV1 {
    Ready,
    PendingPrepare,
    Prepared,
    PendingCommit,
    ReconciliationRequired,
    PendingReconciliation,
    AwaitingAdoption,
    Adopted,
    Rejected,
}
#[derive(Debug)]
pub struct CompleteReconnectFlowV1 {
    operation: CompleteReconnectDurabilityOperationV1,
    authorization: Option<CompleteReconnectAuthorizationV1>,
    claims: Option<super::admission_authority_publication::CompleteReconnectClaimTransitionV1>,
    phase: CompleteReconnectPhaseV1,
    prepared_at: Option<i64>,
    receipt: Option<CompleteReconnectReceiptV1>,
}
impl CompleteReconnectFlowV1 {
    pub fn begin(
        authorization: CompleteReconnectAuthorizationV1,
        claims: Option<super::admission_authority_publication::CompleteReconnectClaimTransitionV1>,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        let operation = CompleteReconnectDurabilityOperationV1 {
            recovery: authorization.operation().clone(),
            replacement: claims.as_ref().map(|value| value.evidence().clone()),
        };
        operation.validate_historical()?;
        Ok(Self {
            operation,
            authorization: Some(authorization),
            claims,
            phase: CompleteReconnectPhaseV1::Ready,
            prepared_at: None,
            receipt: None,
        })
    }
    pub fn restore(
        operation: CompleteReconnectDurabilityOperationV1,
    ) -> Result<Self, ReconnectDurabilityErrorV1> {
        operation.validate_historical()?;
        Ok(Self {
            operation,
            authorization: None,
            claims: None,
            phase: CompleteReconnectPhaseV1::ReconciliationRequired,
            prepared_at: None,
            receipt: None,
        })
    }
    #[must_use]
    pub const fn phase(&self) -> CompleteReconnectPhaseV1 {
        self.phase
    }
    #[must_use]
    pub const fn operation(&self) -> &CompleteReconnectDurabilityOperationV1 {
        &self.operation
    }
    #[must_use]
    pub const fn receipt(&self) -> Option<&CompleteReconnectReceiptV1> {
        self.receipt.as_ref()
    }
    pub fn take_request(
        &mut self,
        kind: CompleteReconnectRequestKindV1,
    ) -> Result<CompleteReconnectRequestV1, ReconnectDurabilityErrorV1> {
        let phase = match (self.phase, kind) {
            (CompleteReconnectPhaseV1::Ready, CompleteReconnectRequestKindV1::Prepare) => {
                CompleteReconnectPhaseV1::PendingPrepare
            }
            (CompleteReconnectPhaseV1::Prepared, CompleteReconnectRequestKindV1::Commit) => {
                CompleteReconnectPhaseV1::PendingCommit
            }
            (
                CompleteReconnectPhaseV1::ReconciliationRequired,
                CompleteReconnectRequestKindV1::Reconcile,
            ) => CompleteReconnectPhaseV1::PendingReconciliation,
            _ => return Err(ReconnectDurabilityErrorV1::InvalidPhase),
        };
        if kind != CompleteReconnectRequestKindV1::Reconcile && self.authorization.is_none() {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        self.phase = phase;
        Ok(CompleteReconnectRequestV1 {
            kind,
            operation: self.operation.clone(),
            authorization: if kind == CompleteReconnectRequestKindV1::Reconcile {
                None
            } else {
                self.authorization.take()
            },
            claims: if kind == CompleteReconnectRequestKindV1::Reconcile {
                None
            } else {
                self.claims.take()
            },
        })
    }
    /// Resuming any prepared write requires reauthorization from fresh sources.
    pub fn resume_prepared(
        &mut self,
        authorization: CompleteReconnectAuthorizationV1,
        source: &dyn CompleteReconnectSourceV1,
        now: i64,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase != CompleteReconnectPhaseV1::Prepared
            || authorization.operation() != &self.operation.recovery
        {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        authorization.validate_current(source, true, now)?;
        let claims = match &self.operation.replacement {
            Some(evidence) => Some(
                super::admission_authority_publication::CompleteReconnectClaimTransitionV1::resume(
                    evidence.clone(),
                    &authorization,
                    now,
                )
                .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?,
            ),
            None => None,
        };
        self.authorization = Some(authorization);
        self.claims = claims;
        Ok(())
    }
    pub fn mark_ambiguous(&mut self) -> Result<(), ReconnectDurabilityErrorV1> {
        if !matches!(
            self.phase,
            CompleteReconnectPhaseV1::PendingPrepare
                | CompleteReconnectPhaseV1::PendingCommit
                | CompleteReconnectPhaseV1::PendingReconciliation
        ) {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        self.phase = CompleteReconnectPhaseV1::ReconciliationRequired;
        Ok(())
    }
    pub fn accept_completion(
        &mut self,
        source: &mut dyn CompleteReconnectCompletionSourceV1,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        if self.phase == CompleteReconnectPhaseV1::Ready {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let Some(completion) = source.take_complete_reconnect_completion(&self.operation)? else {
            return Ok(());
        };
        completion.operation.validate_historical()?;
        if completion.operation != self.operation {
            return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
        }
        match completion.outcome {
            CompleteReconnectOutcomeV1::Prepared { decided_at } => {
                complete_reconnect_effect(&self.operation, false, decided_at)?;
                if self.phase == CompleteReconnectPhaseV1::Rejected
                    || self
                        .receipt
                        .as_ref()
                        .is_some_and(|receipt| decided_at > receipt.decided_at)
                    || self.prepared_at.is_some_and(|prior| prior != decided_at)
                {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                self.prepared_at = Some(decided_at);
                if !matches!(
                    self.phase,
                    CompleteReconnectPhaseV1::AwaitingAdoption | CompleteReconnectPhaseV1::Adopted
                ) {
                    self.phase = CompleteReconnectPhaseV1::Prepared;
                }
            }
            CompleteReconnectOutcomeV1::Committed { decided_at } => {
                complete_reconnect_effect(&self.operation, true, decided_at)?;
                if self.phase == CompleteReconnectPhaseV1::Rejected
                    || self.prepared_at.is_some_and(|prior| prior > decided_at)
                {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                let receipt = CompleteReconnectReceiptV1 {
                    operation: self.operation.clone(),
                    decided_at,
                };
                if self.receipt.as_ref().is_some_and(|prior| prior != &receipt) {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                self.receipt = Some(receipt);
                if self.phase != CompleteReconnectPhaseV1::Adopted {
                    self.phase = CompleteReconnectPhaseV1::AwaitingAdoption;
                }
            }
            CompleteReconnectOutcomeV1::Rejected => {
                if self.receipt.is_some() {
                    return Err(ReconnectDurabilityErrorV1::IdempotencyConflict);
                }
                self.phase = CompleteReconnectPhaseV1::Rejected;
            }
            CompleteReconnectOutcomeV1::Ambiguous => {
                if !matches!(
                    self.phase,
                    CompleteReconnectPhaseV1::AwaitingAdoption
                        | CompleteReconnectPhaseV1::Adopted
                        | CompleteReconnectPhaseV1::Rejected
                ) {
                    self.phase = CompleteReconnectPhaseV1::ReconciliationRequired;
                }
            }
        }
        Ok(())
    }
}
/// Independently current post-COMMIT snapshot. All controller/actor/claim facts
/// come from this source; a historical winner alone cannot install control.
pub trait CompleteReconnectAdoptionSourceV1: CompleteReconnectSourceV1 {
    /// Return only an independently active, non-revoked proof-owner projection;
    /// stored reservation metadata is not current activation evidence.
    fn current_reconnect_proof(
        &self,
        _identity: &ReconnectIdentityV1,
        _now: i64,
    ) -> Result<CompleteReconnectProofCurrentV1, ReconnectDurabilityErrorV1> {
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    }

    fn current_fast_reconnect_proof(
        &self,
        _identity: &ReconnectIdentityV1,
        _now: i64,
    ) -> Result<CompleteFastReconnectAdoptionV1, ReconnectDurabilityErrorV1> {
        Err(ReconnectDurabilityErrorV1::StaleAuthority)
    }
    fn resolve_complete_reconnect_adoption(
        &self,
        identity: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<CompleteReconnectSnapshotV1, ReconnectDurabilityErrorV1>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompleteReconnectControllerV1 {
    session: GameSessionId,
    transport: AuthenticatedTransportRefV1,
    generation: ConnectionGeneration,
}
impl CompleteReconnectControllerV1 {
    #[must_use]
    pub const fn session(&self) -> GameSessionId {
        self.session
    }
    #[must_use]
    pub const fn transport(&self) -> AuthenticatedTransportRefV1 {
        self.transport
    }
    #[must_use]
    pub const fn generation(&self) -> ConnectionGeneration {
        self.generation
    }
}
impl CompleteReconnectFlowV1 {
    pub fn adopt_current(
        &mut self,
        source: &dyn CompleteReconnectAdoptionSourceV1,
        now: i64,
    ) -> Result<CompleteReconnectControllerV1, ReconnectDurabilityErrorV1> {
        if !matches!(
            self.phase,
            CompleteReconnectPhaseV1::AwaitingAdoption | CompleteReconnectPhaseV1::Adopted
        ) {
            return Err(ReconnectDurabilityErrorV1::InvalidPhase);
        }
        let receipt = self
            .receipt
            .as_ref()
            .ok_or(ReconnectDurabilityErrorV1::InvalidPhase)?;
        let current =
            source.resolve_complete_reconnect_adoption(&self.operation.recovery.identity, now)?;
        current.validate_resources()?;
        let proof = source.current_reconnect_proof(&self.operation.recovery.identity, now)?;
        let transition = &self.operation.recovery.original.proof_transition;
        if proof.owner != transition.owner
            || proof.revision <= transition.revision
            || proof.accepted_revision != proof.revision
            || proof.observed_at < receipt.decided_at
            || proof.observed_at > now
            || proof
                .observed_at
                .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                .is_none_or(|deadline| now > deadline)
            || proof.session != transition.successor_session
            || proof.proof_generation != transition.successor_generation
            || proof.connection != transition.candidate.connection_generation()
            || proof.transport != transition.candidate.transport_ref()
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        validate_complete_reconnect_adoption_trust(
            &self.operation.recovery,
            source,
            receipt.decided_at,
            now,
        )?;
        let expected = complete_reconnect_effect(&self.operation, true, receipt.decided_at)?;
        let original = &self.operation.recovery.original;
        if now < receipt.decided_at
            || current.observed_at > now
            || current.observed_at < receipt.decided_at
            || current
                .observed_at
                .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                .is_none_or(|deadline| now > deadline)
            || current.source_authority != original.source_authority
            || current.source_revision < original.source_revision
            || current.accepted_source_revision != current.source_revision
            || !current.actor_present
            || !current.runtime_ready
            || current.session != expected.session
            || current.protection != expected.protection
            || current.budget != expected.budget
            || current.claims != expected.claims
            || current.loss != original.loss
            || current.loss_decided_at != original.loss_decided_at
            || current.account_presence != original.account_presence
            || current.recovery != original.recovery
            || current.placement_identity != original.placement_identity
            || current.placement_revision != original.placement_revision
            || current.fnd02 != original.fnd02
            || current.candidate != original.candidate
            || current.proof_transition != original.proof_transition
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        super::admission_authority_publication::validate_complete_reconnect_claims(
            self.operation.recovery.identity.account_id(),
            current.session,
            &current.claims,
            now,
        )
        .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;
        validate_current_authority(
            self.operation.recovery.identity.game_session_id(),
            current.session,
        )
        .map_err(|_| ReconnectDurabilityErrorV1::StaleAuthority)?;
        self.phase = CompleteReconnectPhaseV1::Adopted;
        Ok(CompleteReconnectControllerV1 {
            session: self.operation.recovery.identity.game_session_id(),
            transport: original.candidate.transport_ref(),
            generation: original.candidate.connection_generation(),
        })
    }
}

fn validate_complete_reconnect_adoption_trust(
    operation: &CompleteReconnectOperationV1,
    source: &dyn CompleteReconnectAdoptionSourceV1,
    decided_at: i64,
    now: i64,
) -> Result<(), ReconnectDurabilityErrorV1> {
    use super::fnd04_verifier::{Fnd04EvidenceScope, validate_recovery_adoption_sources};
    let stale = ReconnectDurabilityErrorV1::StaleAuthority;
    let Some(credential) = operation.credential.recovery() else {
        let CompleteReconnectCredentialV1::Fast(original) = &operation.credential else {
            return Err(stale);
        };
        let current = source.current_fast_reconnect_proof(&operation.identity, now)?;
        let minimum = source
            .recovery_v1_authority()
            .ok_or(stale)?
            .account_minimum_generation(
                Fnd04EvidenceScope::ExistingActorRecovery,
                operation.identity.account_id(),
                now,
            )
            .map_err(|_| stale)?;
        if minimum == 0
            || minimum > original.compatibility.account_security_generation()
            || current.observed_at < decided_at
            || current
                .observed_at
                .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                .is_none_or(|deadline| now > deadline)
        {
            return Err(stale);
        }
        return current.validate(operation, original, now);
    };
    match (&credential.v1_trust, &credential.v2) {
        (Some(prior), None) => {
            let authority = source.recovery_v1_authority().ok_or(stale)?;
            let key = authority
                .signing_key(
                    Fnd04EvidenceScope::ExistingActorRecovery,
                    &prior.signing_key_id,
                    now,
                )
                .map_err(|_| stale)?;
            let minimum = authority
                .account_minimum_generation(
                    Fnd04EvidenceScope::ExistingActorRecovery,
                    operation.identity.account_id(),
                    now,
                )
                .map_err(|_| stale)?;
            if key != prior.signing_public_key
                || minimum < prior.minimum_generation
                || minimum > credential.account_security_generation
            {
                return Err(stale);
            }
            Ok(())
        }
        (None, Some(prior)) => {
            let authority = source.recovery_v2_source().ok_or(stale)?;
            let signing = authority
                .signing_trust(&prior.signing.key_id, now)
                .map_err(|_| stale)?;
            let security = authority
                .account_security(operation.identity.account_id(), now)
                .map_err(|_| stale)?;
            validate_recovery_adoption_sources(prior, &signing, &security, now).map_err(|_| stale)
        }
        _ => Err(stale),
    }
}

/// Current proof-owner projection after the committed rotation, not a bearer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteFastReconnectAdoptionV1 {
    pub session: GameSessionId,
    pub connection: ConnectionGeneration,
    pub transport: AuthenticatedTransportRefV1,
    pub proof_generation: u64,
    pub observed_at: i64,
    pub compatibility: ReconnectCompatibilityEvidenceV1,
}
impl CompleteFastReconnectAdoptionV1 {
    fn validate(
        &self,
        operation: &CompleteReconnectOperationV1,
        original: &CompleteFastReconnectBindingV1,
        now: i64,
    ) -> Result<(), ReconnectDurabilityErrorV1> {
        let stale = ReconnectDurabilityErrorV1::StaleAuthority;
        let current = &self.compatibility;
        let prior = &original.compatibility;
        if self.session != operation.identity.game_session_id()
            || self.connection != operation.original.candidate.connection_generation()
            || self.transport != operation.original.candidate.transport_ref()
            || self.proof_generation != original.replacement_proof_generation
            || self.observed_at > now
            || self.observed_at < original.verified_at
            || current.account_security_generation() != prior.account_security_generation()
            || current.protocol_major() != prior.protocol_major()
            || current.transport_profile() != prior.transport_profile()
            || current.ruleset_revision() != prior.ruleset_revision()
            || current.content_revision() != prior.content_revision()
            || current.map_revision() != prior.map_revision()
            || current.world_policy_revision() != prior.world_policy_revision()
            || current.credential_expiration().is_some()
        {
            return Err(stale);
        }
        for (before, after) in [
            (
                prior.platform_security_evidence(),
                current.platform_security_evidence(),
            ),
            (prior.proof_trust_evidence(), current.proof_trust_evidence()),
        ] {
            if after.authority() != before.authority()
                || after.purpose() != before.purpose()
                || after.scope() != before.scope()
                || after.source_observed_at() < before.source_observed_at()
                || after.source_observed_at() > now
                || after
                    .source_observed_at()
                    .checked_add(EVIDENCE_FRESHNESS_SECONDS_V1)
                    .is_none_or(|deadline| now > deadline)
            {
                return Err(stale);
            }
        }
        Ok(())
    }
}
